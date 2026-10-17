use crate::{
    OpaConfig, PolicyArtifactRef, PolicyDecision, PolicyEvaluation, PolicyInput, PolicyPackage,
    PolicyProcessObservation, PolicyReceipt, paths, policy_package::hash,
    policy_workspace::PolicyWorkspace,
};
use guard_protocol::ExecutionStatus;
use guard_runner::{Cancellation, CommandSpec, ProcessRunner, RunOutput, StopReason};
use guard_store::EvidenceStore;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
pub(crate) fn now() -> Result<u64, String> {
    u64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_millis(),
    )
    .map_err(|e| e.to_string())
}
pub(crate) fn save(
    store: &EvidenceStore,
    operation: &str,
    bytes: &[u8],
    retain: u64,
) -> Result<PolicyArtifactRef, String> {
    store.put(operation, bytes, retain, now()?)?;
    Ok(PolicyArtifactRef {
        operation: operation.into(),
        digest: hash(bytes),
        size: bytes.len() as u64,
    })
}
fn not_run() -> RunOutput {
    RunOutput {
        status: ExecutionStatus::NotRun,
        exit_code: None,
        reason: StopReason::IoFailure,
        stdout: vec![],
        stderr: vec![],
        output_truncated: false,
        process_id: None,
        started_at: None,
        finished_at: None,
    }
}
fn observe(
    store: &EvidenceStore,
    operation: &str,
    phase: &str,
    out: &RunOutput,
    retain: u64,
) -> Result<PolicyProcessObservation, String> {
    Ok(PolicyProcessObservation {
        status: out.status,
        exit_code: out.exit_code,
        started_at: out.started_at.clone(),
        finished_at: out.finished_at.clone(),
        stop_reason: if out.status == ExecutionStatus::NotRun {
            "InputRejected".into()
        } else {
            format!("{:?}", out.reason)
        },
        output_truncated: out.output_truncated,
        stdout: save(
            store,
            &format!("{operation}:{phase}:stdout"),
            &out.stdout,
            retain,
        )?,
        stderr: save(
            store,
            &format!("{operation}:{phase}:stderr"),
            &out.stderr,
            retain,
        )?,
    })
}
fn version_matches(out: &RunOutput, expected: &str) -> bool {
    if out.status != ExecutionStatus::Completed
        || out.exit_code != Some(0)
        || out.output_truncated
        || out.reason != StopReason::Exited
    {
        return false;
    }
    let Ok(text) = std::str::from_utf8(&out.stdout) else {
        return false;
    };
    let platform = match (std::env::consts::OS, std::env::consts::ARCH) {
        ("windows", "x86_64") => "windows/amd64",
        ("linux", "x86_64") => "linux/amd64",
        ("linux", "aarch64") => "linux/arm64",
        _ => return false,
    };
    let lines = text
        .lines()
        .map(|s| s.trim_end_matches('\r'))
        .collect::<Vec<_>>();
    lines.first() == Some(&format!("Version: {expected}").as_str())
        && lines.contains(&"Rego Version: v1")
        && lines.contains(&format!("Platform: {platform}").as_str())
}
/// 通用本地OPA执行；源文件固定成私有副本，输出始终未授信，不拥有任何专业策略。无Java对应类型。
#[derive(Clone)]
pub struct OpaEvaluator {
    pub(crate) config: OpaConfig,
    pub(crate) package: PolicyPackage,
    pub(crate) store: Arc<EvidenceStore>,
    scratch: PathBuf,
    retain: u64,
    runner: ProcessRunner,
}
impl OpaEvaluator {
    /// 创建显式本地执行器；binary hash在每次K06进程前后检查，不自动安装或根据version文本授信。
    pub fn new(
        config: OpaConfig,
        package: PolicyPackage,
        store: Arc<EvidenceStore>,
        scratch_parent: &Path,
        retain_until_ms: u64,
    ) -> Result<Self, String> {
        config.configuration_digest(&package)?;
        let scratch = paths::safe(scratch_parent)?;
        if !scratch.is_dir() {
            return Err("existing private policy scratch parent required".into());
        }
        let runner = ProcessRunner::new(config.budget.clone())?;
        Ok(Self {
            config,
            package,
            store,
            scratch,
            retain: retain_until_ms,
            runner,
        })
    }
    /// 核验fixed版本并以literal argv读取私有包和输入；undefined/错误/超时/取消/截断均为Incomplete。
    pub async fn evaluate(
        &self,
        input: &PolicyInput,
        operation: &str,
        cancel: Cancellation,
    ) -> Result<PolicyReceipt, String> {
        self.evaluate_until(
            input,
            operation,
            cancel,
            Instant::now() + Duration::from_millis(self.config.budget.timeout_ms),
        )
        .await
    }
    pub(crate) async fn evaluate_until(
        &self,
        input: &PolicyInput,
        operation: &str,
        cancel: Cancellation,
        deadline: Instant,
    ) -> Result<PolicyReceipt, String> {
        let input = PolicyInput::from_json(&input.bytes()?)?;
        if input.binding().policy_digest != *self.package.digest() {
            return Err("input policy digest differs from immutable package".into());
        }
        // 先持久化单次消费意图；即使future被丢弃也不静默重复运行同operation。这不是授权或完整RunLedger。
        let intent=guard_protocol::canonicalize(&serde_json::to_vec(&serde_json::json!({"schemaVersion":"guard.policy-intent/v1","operation":operation,"binding":input.binding(),"inputDigest":input.digest()?,"policyDigest":self.package.digest(),"configDigest":self.configuration_digest()?})).map_err(|e|e.to_string())?)?;
        let intent_operation = format!("{operation}:intent");
        save(&self.store, &intent_operation, &intent, self.retain)?;
        let key = format!(
            "guard-policy-operation:{}",
            hash(operation.as_bytes()).as_str()
        );
        match self.store.compare_and_swap_head(&key,None,&intent_operation,now()?)?{guard_store::HeadUpdate::Applied(_)=>{},_=>return Err("policy operation already consumed; retain crash intent and use a new explicit operation".into())}
        let package = self.package.clone();
        let private_input = input.clone();
        let scratch = self.scratch.clone();
        let workspace = match crate::blocking::run_until(deadline, move || {
            PolicyWorkspace::create(&scratch, &package, &private_input)
        })
        .await
        {
            Ok(value) => Arc::new(value),
            Err(error) => {
                let mut phase = not_run();
                if Instant::now() >= deadline {
                    phase.status = ExecutionStatus::TimedOut;
                    phase.reason = StopReason::Timeout;
                }
                return self.finish(
                    &input,
                    operation,
                    phase,
                    not_run(),
                    vec![format!("policy preparation incomplete: {error}")],
                    None,
                );
            }
        };
        let root = paths::native(workspace.root())?;
        let exe = paths::native(&paths::safe(&self.config.executable)?)?;
        let spec = |args: Vec<String>| CommandSpec {
            executable: exe.clone(),
            executable_digest: self.config.executable_digest.clone(),
            args,
            directory: root.clone(),
            environment: self.config.environment.clone(),
        };
        let mut diagnostics = Vec::new();
        let version = match self
            .runner
            .run_until(spec(vec!["version".into()]), cancel.clone(), deadline)
            .await
        {
            Ok(out) => out,
            Err(error) => {
                diagnostics.push(format!("OPA version process rejected: {error}"));
                not_run()
            }
        };
        let version_ok = version_matches(&version, self.config.version.as_str());
        let mut execution = not_run();
        if version_ok {
            let path = |name: &str| {
                paths::native(&workspace.path(name)).and_then(|p| {
                    p.to_str()
                        .map(str::to_string)
                        .ok_or("Unicode private policy path required".into())
                })
            };
            let args = vec![
                "eval".into(),
                "--format=json".into(),
                "--strict".into(),
                "--strict-builtin-errors".into(),
                "--fail".into(),
                "--target=rego".into(),
                format!("--capabilities={}", path("capabilities.json")?),
                format!("--bundle={}", path("bundle")?),
                format!("--input={}", path("input.json")?),
                format!("--timeout={}ms", self.config.budget.timeout_ms),
                self.package.manifest().entrypoint.clone(),
            ];
            match self.runner.run_until(spec(args), cancel, deadline).await {
                Ok(out) => execution = out,
                Err(error) => diagnostics.push(format!("OPA evaluation process rejected: {error}")),
            }
        } else {
            diagnostics.push("pinned OPA version/platform/Rego probe did not complete exactly; no policy dispatched".into());
        }
        let mut decision = None;
        if execution.status == ExecutionStatus::Completed
            && execution.exit_code == Some(0)
            && execution.reason == StopReason::Exited
            && !execution.output_truncated
        {
            match PolicyDecision::from_opa_json(
                &execution.stdout,
                &self.package.manifest().entrypoint,
            ) {
                Ok(value) => decision = Some(value),
                Err(error) => diagnostics.push(error),
            }
        } else {
            diagnostics.push(format!(
                "OPA result incomplete: {:?}/{:?}/exit{:?}/truncated{}",
                execution.status, execution.reason, execution.exit_code, execution.output_truncated
            ));
        }
        let verify = workspace.clone();
        if let Err(error) = crate::blocking::run(move || verify.verify()).await {
            diagnostics.push(error);
            decision = None;
        }
        self.finish(&input, operation, version, execution, diagnostics, decision)
    }
    #[allow(clippy::too_many_arguments)]
    fn finish(
        &self,
        input: &PolicyInput,
        operation: &str,
        version: RunOutput,
        execution: RunOutput,
        diagnostics: Vec<String>,
        decision: Option<PolicyDecision>,
    ) -> Result<PolicyReceipt, String> {
        let input_ref = save(
            &self.store,
            &format!("{operation}:input"),
            &input.bytes()?,
            self.retain,
        )?;
        let mut sources = vec![
            save(
                &self.store,
                &format!("{operation}:manifest"),
                &guard_protocol::canonicalize(
                    &serde_json::to_vec(self.package.manifest()).map_err(|e| e.to_string())?,
                )?,
                self.retain,
            )?,
            save(
                &self.store,
                &format!("{operation}:capabilities"),
                self.package.capabilities(),
                self.retain,
            )?,
        ];
        for (index, bytes) in self.package.files().values().enumerate() {
            sources.push(save(
                &self.store,
                &format!("{operation}:policy:{index}"),
                bytes,
                self.retain,
            )?);
        }
        let evaluation = PolicyEvaluation {
            schema_version: "guard.policy-evaluation/v1",
            binding: input.binding().clone(),
            input_digest: input.digest()?,
            policy_digest: self.package.digest().clone(),
            config_digest: self.config.configuration_digest(&self.package)?,
            incomplete: decision.is_none(),
            decision,
            version: observe(&self.store, operation, "version", &version, self.retain)?,
            execution: observe(&self.store, operation, "eval", &execution, self.retain)?,
            diagnostics,
            input: input_ref,
            policy_sources: sources,
            trust: "UNVERIFIED",
        };
        let bytes = guard_protocol::canonicalize(
            &serde_json::to_vec(&evaluation).map_err(|e| e.to_string())?,
        )?;
        let artifact = save(
            &self.store,
            &format!("{operation}:metadata"),
            &bytes,
            self.retain,
        )?;
        Ok(PolicyReceipt {
            evaluation,
            artifact,
        })
    }
    /// 返回当前固定配置与包身份；不授予源包作者或调用者信任。
    pub fn configuration_digest(&self) -> Result<guard_protocol::Digest, String> {
        self.config.configuration_digest(&self.package)
    }
    pub(crate) fn retain(&self) -> u64 {
        self.retain
    }
}
