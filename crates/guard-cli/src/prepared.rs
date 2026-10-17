use crate::{
    configured_verifier::ConfiguredVerifier,
    file_ports::FilePorts,
    host_config::HostConfig,
    probe_results::ProbeResults,
    safe_files::{decode, get},
};
use guard_authority::{ApprovalVerifier, ReceiptApprovalAuthority};
use guard_contract::{
    CompileContext, CompileRequest, CompiledContract, ContractLayer, LayerLevel, SourceMaterial,
    VerifierRegistry,
};
use guard_protocol::{ExecutionBudget, Identifier, ProtocolDocument};
use guard_runner::{Cancellation, ProcessRunner};
use guard_store::{EvidenceStore, StoreLimits};
use guard_subject::{CaptureBinding, CapturedSubject, Selection};
use serde_json::{Value, json};
use std::{collections::BTreeMap, path::PathBuf, sync::Arc};
/// 当前真实编译产物与宿主配置；不可从命令行JSON恢复内部对象。无Java对应类型。
pub(crate) struct Prepared {
    pub host: HostConfig,
    pub contract: CompiledContract,
    pub package: guard_policy::PolicyPackage,
    pub verifiers: Vec<ConfiguredVerifier>,
}
/// 读取宿主UTC时钟，返回非负毫秒或错误。
pub(crate) fn now() -> Result<u64, String> {
    u64::try_from(chrono::Utc::now().timestamp_millis()).map_err(|e| e.to_string())
}
/// 按宿主引用加载实际政策包及摘要，返回核验包。
pub(crate) fn package(host: &HostConfig) -> Result<guard_policy::PolicyPackage, String> {
    let root = host.policy_root()?;
    guard_policy::PolicyPackage::load(
        &root,
        &crate::safe_files::read(&root.join("manifest.json"), 1024 * 1024)?,
    )
}
/// 读取有界验证器清单，返回显式固定配置。
pub(crate) fn configs(host: &HostConfig) -> Result<Vec<ConfiguredVerifier>, String> {
    let values: Vec<ConfiguredVerifier> = host.field("verifiers")?;
    if values.is_empty() || values.len() > 64 {
        return Err("bounded nonempty verifier list required".into());
    }
    Ok(values)
}
/// 对实际Git描述和平台生成稳定JCS摘要，返回环境标记身份。
pub(crate) fn git_identity(host: &HostConfig) -> Result<guard_protocol::Digest, String> {
    guard_protocol::digest_json(&serde_json::to_vec(&json!({"schemaVersion":"guard.cli-git-provider/v1","platform":{"os":std::env::consts::OS,"arch":std::env::consts::ARCH},"provider":host.data.get("git").ok_or("fixed Git provider missing")?})).map_err(|e|e.to_string())?)
}
fn environment_identity(
    host: &HostConfig,
    values: &[ConfiguredVerifier],
) -> Result<guard_protocol::Digest, String> {
    let expected_git = git_identity(host)?;
    let baseline: guard_protocol::Digest = host.field("environmentDigest")?;
    let mut result = None;
    for native in values.iter().filter(|v| v.kind == "domain") {
        if native
            .environment
            .get("GUARDCORE_GIT_PROVIDER_DIGEST")
            .map(String::as_str)
            != Some(expected_git.as_str())
            || native
                .environment
                .get("GUARDCORE_ENVIRONMENT_BASELINE_DIGEST")
                .map(String::as_str)
                != Some(baseline.as_str())
        {
            return Err("approved native environment does not bind actual Git provider and external baseline".into());
        }
        let actual =
            guard_runtime::NativeVerifierCommandV2::environment_digest(&native.environment)?;
        if result.as_ref().is_some_and(|old| old != &actual) {
            return Err("one shared approved native environment manifest required".into());
        }
        result = Some(actual);
    }
    result.ok_or("native environment manifest missing".into())
}
/// 在空私有目录执行固定版本探测，返回实际注册能力与观察。
pub(crate) async fn probe(
    host: &HostConfig,
) -> Result<(VerifierRegistry, ProbeResults, Value), String> {
    let values = configs(host)?;
    environment_identity(host, &values)?;
    let policy = values
        .iter()
        .filter(|v| v.kind == "policy_engine")
        .collect::<Vec<_>>();
    if policy.len() != 1 {
        return Err("one policy engine required".into());
    }
    let mut registry = VerifierRegistry::new(policy[0].verifier_id.clone());
    let mut observed = BTreeMap::new();
    let mut report = Vec::new();
    let root = host.project()?;
    let package = package(host)?;
    let scratch = crate::safe_files::plain(&host.field::<PathBuf>("scratchRoot")?)?;
    let private = tempfile::Builder::new()
        .prefix("guardcore-probe-")
        .tempdir_in(&scratch)
        .map_err(|e| e.to_string())?;
    // 探测只在空私有cwd使用独立最小环境，不使用候选cwd或真实run环境。
    let runner = ProcessRunner::new(ExecutionBudget {
        timeout_ms: 30000,
        max_output_bytes: 65536,
        max_concurrency: 1,
    })?;
    for verifier in values {
        verifier.validate_probe()?;
        registry.register(verifier.descriptor()?)?;
        let pin = registry
            .pin(&verifier.verifier_id)
            .ok_or("missing registered pin")?
            .clone();
        let config = if verifier.kind == "domain" {
            guard_runtime::NativeVerifierCommandV2::configuration_digest(
                &verifier.command(&root, false),
            )?
        } else {
            opa_config(host, &verifier)?.configuration_digest(&package)?
        };
        let outcome = if config != verifier.config_digest {
            Err("configuration differs from approved pin".to_string())
        } else {
            match runner
                .run(verifier.command(private.path(), true), Cancellation::new())
                .await
            {
                Ok(out)
                    if out.status == guard_protocol::ExecutionStatus::Completed
                        && out.exit_code == Some(0)
                        && !out.output_truncated
                        && out.stdout == verifier.probe_stdout.as_bytes()
                        && out.stderr == verifier.probe_stderr.as_bytes() =>
                {
                    Ok(pin)
                }
                Ok(out) => Err(format!(
                    "fixed read-only probe incomplete: {:?}/{:?}/exit{:?}",
                    out.status, out.reason, out.exit_code
                )),
                Err(e) => Err(e),
            }
        };
        report.push(json!({"verifierId":verifier.verifier_id,"artifactVerified":outcome.is_ok(),"versionOutputMatched":outcome.is_ok(),"domainRunTested":false,"diagnostic":outcome.as_ref().err()}));
        observed.insert(verifier.verifier_id, outcome);
    }
    Ok((registry, ProbeResults(observed), json!(report)))
}
/// 从宿主验证器及预算生成OPA配置，返回显式执行档位。
pub(crate) fn opa_config(
    host: &HostConfig,
    value: &ConfiguredVerifier,
) -> Result<guard_policy::OpaConfig, String> {
    Ok(guard_policy::OpaConfig {
        executable: value.executable.clone(),
        executable_digest: value.artifact_digest.clone(),
        version: value.version.clone(),
        environment: value.environment.clone(),
        budget: host.field("budget")?,
    })
}
impl Prepared {
    /// 从宿主三层原件、真实批准与实际能力编译，返回不可伪造编译产物。
    pub(crate) async fn compile(host: HostConfig) -> Result<Self, String> {
        let request = request(&host)?;
        let ports = Arc::new(FilePorts(host.clone()));
        let verifier = Arc::new(ApprovalVerifier::new(
            ports.clone(),
            ports.clone(),
            ports.clone(),
            ports,
        ));
        let authority = ReceiptApprovalAuthority::for_compile(
            &request,
            verifier,
            host.field("audience")?,
            host.field("actor")?,
            None,
        )?;
        let (registry, probes, _) = probe(&host).await?;
        let result = guard_contract::compile(&request, &registry, &probes, Some(&authority))?;
        let contract = result.compiled().cloned().ok_or_else(|| {
            format!(
                "contract blocked: {}",
                serde_json::to_string(result.diagnostics()).unwrap_or_default()
            )
        })?;
        Ok(Self {
            package: package(&host)?,
            verifiers: configs(&host)?,
            host,
            contract,
        })
    }
    /// 按选择文件采集真实Git输入，返回当前候选及完整绑定。
    pub(crate) fn capture(&self, file: &str) -> Result<CapturedSubject, String> {
        let binding = self.host.binding()?;
        if &binding.contract_digest != self.contract.digest() {
            return Err("host TaskBinding differs from freshly approved compiled contract".into());
        }
        let selection: Selection =
            decode(&crate::safe_files::read(&PathBuf::from(file), 1024 * 1024)?)?;
        let capture = CaptureBinding {
            repository_id: self.contract.context().repository_id.clone(),
            task_id: self.contract.context().task_id.clone(),
            worktree_id: binding.worktree_id.clone(),
            contract_digest: self.contract.digest().clone(),
            policy_digest: self.package.digest().clone(),
            verifier_digest: guard_plan::verifier_digest(&self.contract)?,
            environment_digest: environment_identity(&self.host, &self.verifiers)?,
        };
        guard_subject::capture(&self.host.project()?, &capture, &selection)
    }
    /// 按真实候选生成冻结计划或缺失义务，返回规划结果。
    pub(crate) fn freeze(&self, file: &str) -> Result<guard_plan::PlanningOutcome, String> {
        guard_plan::freeze(
            &self.host.project()?,
            &self.contract,
            &self.host.binding()?,
            &self.capture(file)?,
            self.host.field("budget")?,
        )
    }
    /// 按显式创建标志打开受管Store，返回句柄或分类故障。
    pub(crate) fn store(
        &self,
        create: bool,
    ) -> Result<Arc<EvidenceStore>, crate::cli_failure::CliFailure> {
        store(&self.host, create)
    }
    /// 按原件操作名与真实当前候选恢复注册计划，返回当前资格。
    pub(crate) fn restore(
        &self,
        file: &str,
        operation: &str,
        store: &EvidenceStore,
    ) -> Result<guard_plan::RegisteredPlan, String> {
        guard_plan::restore_current(
            store,
            operation,
            now()?,
            &self.host.project()?,
            &self.contract,
            &self.host.binding()?,
            &self.capture(file)?,
            self.host.field("budget")?,
        )
    }
    /// 返回宿主保留期对应的绝对UTC截止。
    pub(crate) fn retain(&self) -> Result<u64, String> {
        retention(&self.host)
    }
}
/// 检查宿主保留期上限，返回绝对UTC截止。
pub(crate) fn retention(host: &HostConfig) -> Result<u64, String> {
    let duration: u64 = host.field("retentionMs")?;
    if duration == 0 || duration > 30 * 86400000 {
        return Err("bounded host retention required".into());
    }
    now()?
        .checked_add(duration)
        .ok_or("retention overflow".into())
}
/// 按显式创建标志打开受管Store，返回句柄或分类故障。
pub(crate) fn store(
    host: &HostConfig,
    create: bool,
) -> Result<Arc<EvidenceStore>, crate::cli_failure::CliFailure> {
    let root: PathBuf = host.field("stateRoot")?;
    let result = if root.exists() {
        EvidenceStore::open(&root, StoreLimits::default())
    } else if create {
        EvidenceStore::create(&root, StoreLimits::default())
    } else {
        return Err(crate::cli_failure::CliFailure::Incomplete(
            "private store is not initialized; explicit plan register required".into(),
        ));
    };
    Ok(Arc::new(
        result.map_err(crate::cli_failure::CliFailure::Internal)?,
    ))
}
fn request(host: &HostConfig) -> Result<CompileRequest, String> {
    let inputs: Vec<Value> = host.field("layers")?;
    let mut layers = Vec::new();
    for input in inputs {
        crate::file_ports::closed(&input, &["level", "document", "sources"])?;
        let level = match get::<String>(&input, "level")?.as_str() {
            "organization" => LayerLevel::Organization,
            "project" => LayerLevel::Project,
            "task" => LayerLevel::Task,
            _ => return Err("unknown layer".into()),
        };
        let document = host.file(&get::<String>(&input, "document")?)?;
        let decoded = guard_protocol::decode(&document)?;
        let ProtocolDocument::ContractSnapshot(snapshot) = decoded.document() else {
            return Err("layer must be ContractSnapshot".into());
        };
        let mapping = input["sources"]
            .as_object()
            .ok_or("explicit source reference map required")?;
        let mut sources = Vec::new();
        for source in &snapshot.source_refs {
            let name = mapping
                .get(source.content_digest.as_str())
                .and_then(Value::as_str)
                .ok_or("source digest file mapping missing")?;
            sources.push(SourceMaterial {
                source: source.clone(),
                bytes: host.file(name)?,
            });
        }
        layers.push(ContractLayer {
            level,
            document,
            sources,
        });
    }
    Ok(CompileRequest {
        layers,
        context: CompileContext {
            repository_id: host.field("repositoryId")?,
            task_id: host.field("taskId")?,
            authorization_refs: host.field::<Vec<Identifier>>("authorizationRefs")?,
        },
        action: host.field("action")?,
    })
}
