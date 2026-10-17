#![cfg(feature = "real-opa-tests")]
use guard_contract::{
    ApprovalAttestation, ApprovalAuthority, CapabilityProbe, CompileContext, CompileRequest,
    CompiledContract, ContractLayer, LayerLevel, SourceMaterial, TaskAuthorization,
    VerifierDescriptor, VerifierKind, VerifierObservation, VerifierRegistry, compile,
    snapshot_digest,
};
use guard_plan::{freeze, verifier_digest};
use guard_policy::{BoundPolicyEvaluator, OpaConfig, PolicyPackage};
use guard_protocol::{
    Action, Digest, DomainOwner, ExecutionBudget, Extensions, GitOid, GitPath, Identifier,
    ProtocolDocument, SchemaVersion, TaskBinding, TaskScope,
};
use guard_runner::Cancellation;
use guard_store::{EvidenceStore, StoreLimits};
use guard_subject::{CaptureBinding, CapturedSubject, Selection, capture};
use serde_json::{Value, json};
use sha2::{Digest as Hasher, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}
fn config() -> OpaConfig {
    let mut environment = BTreeMap::new();
    for key in ["SystemRoot", "TEMP", "TMP"] {
        if let Ok(value) = std::env::var(key) {
            environment.insert(key.into(), value);
        }
    }
    OpaConfig {
        executable: PathBuf::from(std::env::var("GUARD_POLICY_OPA_EXECUTABLE").unwrap()),
        executable_digest: Digest::new(std::env::var("GUARD_POLICY_OPA_SHA256").unwrap()).unwrap(),
        version: Identifier::new("1.21.2").unwrap(),
        environment,
        budget: budget(),
    }
}
fn id(value: &str) -> Identifier {
    Identifier::new(value).unwrap()
}
fn digest(bytes: &[u8]) -> Digest {
    Digest::new(format!("sha256:{:x}", Sha256::digest(bytes))).unwrap()
}
fn git(root: &Path, args: &[&str]) -> String {
    let result = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env(
            "GIT_CONFIG_GLOBAL",
            if cfg!(windows) { "NUL" } else { "/dev/null" },
        )
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    String::from_utf8(result.stdout).unwrap().trim().into()
}
struct Approved {
    layers: BTreeMap<LayerLevel, (Digest, Identifier)>,
    context: CompileContext,
}
impl ApprovalAuthority for Approved {
    fn verify_layer(
        &self,
        context: &CompileContext,
        level: LayerLevel,
        approval: &Identifier,
        digest: &Digest,
        action: Action,
    ) -> Result<ApprovalAttestation, String> {
        if context != &self.context
            || action != Action::Merge
            || self.layers.get(&level) != Some(&(digest.clone(), approval.clone()))
        {
            return Err("not approved".into());
        }
        Ok(ApprovalAttestation {
            authority_id: id("fixture-authority"),
            approval_ref: approval.clone(),
            contract_digest: digest.clone(),
            context: context.clone(),
            level,
            action,
        })
    }
    fn authorize_task(
        &self,
        context: &CompileContext,
        digest: &Digest,
        action: Action,
    ) -> Result<TaskAuthorization, String> {
        if context != &self.context || action != Action::Merge {
            return Err("outside authority scope".into());
        }
        Ok(TaskAuthorization {
            authority_id: id("fixture-authority"),
            authorization_ref: id("AUTH-1"),
            contract_digest: digest.clone(),
            context: context.clone(),
            action,
        })
    }
}
struct Present;
impl CapabilityProbe for Present {
    fn observe(&self, pin: &guard_contract::VerifierPin) -> VerifierObservation {
        VerifierObservation::Available(pin.clone())
    }
}
fn contract(
    bytes: &[u8],
    revision: &str,
    config: &OpaConfig,
    package: &PolicyPackage,
) -> CompiledContract {
    contract_with_protection(bytes, revision, false, config, package)
}
fn contract_with_protection(
    bytes: &[u8],
    revision: &str,
    protected: bool,
    config: &OpaConfig,
    package: &PolicyPackage,
) -> CompiledContract {
    let native = b"approved architecture constraints".to_vec();
    let context = CompileContext {
        repository_id: id("repo-example"),
        task_id: id("REQ-1"),
        authorization_refs: vec![id("AUTH-1")],
    };
    let mut layers = Vec::new();
    let mut approved = BTreeMap::new();
    for level in [
        LayerLevel::Organization,
        LayerLevel::Project,
        LayerLevel::Task,
    ] {
        let mut value: Value = serde_json::from_slice(include_bytes!(
            "../../guard-protocol/fixtures/valid/contract-snapshot.json"
        ))
        .unwrap();
        value["contractId"] = json!(format!("contract-{level:?}"));
        value["version"] = json!(revision);
        value["sourceRefs"][0]["path"] =
            serde_json::to_value(GitPath::from_bytes(b"architecture.json").unwrap()).unwrap();
        value["sourceRefs"][0]["contentDigest"] = json!(digest(&native));
        if protected {
            value["obligations"][0]["targets"][0]["protectedDigest"] = json!(digest(bytes));
        }
        let ProtocolDocument::ContractSnapshot(mut snapshot) =
            guard_protocol::decode(&serde_json::to_vec(&value).unwrap())
                .unwrap()
                .document()
                .clone()
        else {
            panic!("fixture")
        };
        snapshot.digest = snapshot_digest(&snapshot).unwrap();
        approved.insert(
            level,
            (snapshot.digest.clone(), snapshot.approval_ref.clone()),
        );
        let source = snapshot.source_refs[0].clone();
        layers.push(ContractLayer {
            level,
            document: serde_json::to_vec(&ProtocolDocument::ContractSnapshot(snapshot)).unwrap(),
            sources: vec![SourceMaterial {
                source,
                bytes: native.clone(),
            }],
        });
    }
    let mut registry = VerifierRegistry::new(id("opa"));
    for policy in [false, true] {
        registry.register(VerifierDescriptor { verifier_id: id(if policy { "opa" } else { "java-package-check" }), version: if policy{config.version.clone()}else{id("1.0.0")}, artifact_digest: if policy{config.executable_digest.clone()}else{digest(b"binary")}, config_digest: if policy{config.configuration_digest(package).unwrap()}else{digest(b"config")}, adapter_version: if policy{id("guard-opa/v1")}else{id("1.0.0")}, kind: if policy { VerifierKind::PolicyEngine } else { VerifierKind::Domain }, owner: if policy { None } else { Some(DomainOwner::Archguard) }, evidence_types: vec![id(if policy { "policy-decision" } else { "source-report" })], parameter_schema: serde_json::to_vec(&if policy { json!({"type":"object"}) } else { json!({"type":"object","properties":{"maxCycles":{"type":"integer"},"thresholdDecimal":{"type":"string"}},"required":["maxCycles","thresholdDecimal"],"additionalProperties":false}) }).unwrap() }).unwrap();
    }
    let authority = Approved {
        layers: approved,
        context: context.clone(),
    };
    let compiled = compile(
        &CompileRequest {
            layers,
            context,
            action: Action::Merge,
        },
        &registry,
        &Present,
        Some(&authority),
    )
    .unwrap();
    compiled
        .compiled()
        .unwrap_or_else(|| panic!("{:?}", compiled.diagnostics()))
        .clone()
}
fn binding(contract: &CompiledContract) -> TaskBinding {
    TaskBinding {
        schema_version: SchemaVersion::new("1.0.0").unwrap(),
        task_id: id("REQ-1"),
        requirement_refs: contract.source_refs().to_vec(),
        contract_digest: contract.digest().clone(),
        scope: TaskScope {
            repository_ids: vec![id("repo-example")],
            paths: vec![GitPath::from_bytes(b"src/Example.java").unwrap()],
            actions: vec![Action::Merge],
        },
        session_id: id("session-1"),
        worktree_id: id("worktree-1"),
        authorization_refs: vec![id("AUTH-1")],
        depends_on: vec![],
        extensions: Extensions::default(),
    }
}
fn captured(root: &Path, contract: &CompiledContract, package: &PolicyPackage) -> CapturedSubject {
    let base = GitOid::new(git(root, &["rev-parse", "HEAD"])).unwrap();
    capture(
        root,
        &CaptureBinding {
            repository_id: id("repo-example"),
            task_id: id("REQ-1"),
            worktree_id: id("worktree-1"),
            contract_digest: contract.digest().clone(),
            policy_digest: package.digest().clone(),
            verifier_digest: verifier_digest(contract).unwrap(),
            environment_digest: digest(b"jdk17"),
        },
        &Selection::Worktree { base },
    )
    .unwrap()
}
fn budget() -> ExecutionBudget {
    ExecutionBudget {
        timeout_ms: 30000,
        max_output_bytes: 1048576,
        max_concurrency: 2,
    }
}
fn fixture() -> tempfile::TempDir {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    git(root, &["init"]);
    git(root, &["config", "user.name", "Plan Fixture"]);
    git(root, &["config", "user.email", "plan@example.invalid"]);
    git(root, &["config", "core.autocrlf", "false"]);
    fs::create_dir(root.join("src")).unwrap();
    fs::write(root.join("src/Example.java"), b"class Example {}\n").unwrap();
    fs::write(
        root.join("architecture.json"),
        b"approved architecture constraints",
    )
    .unwrap();
    git(root, &["add", "."]);
    git(root, &["commit", "-m", "initial"]);
    temp
}

#[tokio::test]
async fn registered_git_sqlite_plan_pins_real_production_opa_before_and_after() {
    let project = fixture();
    let external = tempfile::tempdir().unwrap();
    let root = PathBuf::from(std::env::var("GUARD_POLICY_FLOWGUARD_PACKAGE").unwrap());
    let package =
        PolicyPackage::load(&root, &fs::read(root.join("manifest.json")).unwrap()).unwrap();
    let config = config();
    let contract = contract(b"class Example {}\n", "1", &config, &package);
    let subject = captured(project.path(), &contract, &package);
    let draft = freeze(
        project.path(),
        &contract,
        &binding(&contract),
        &subject,
        budget(),
    )
    .unwrap();
    let store = Arc::new(
        EvidenceStore::create(&external.path().join("store"), StoreLimits::default()).unwrap(),
    );
    let registered = draft.register(&store, now() + 600000, now()).unwrap();
    let bound = BoundPolicyEvaluator::prepare(
        project.path(),
        &contract,
        &registered,
        config.clone(),
        package.clone(),
        store.clone(),
        external.path(),
        now() + 600000,
    )
    .unwrap();
    let mut bad = config;
    bad.executable_digest = digest(b"unapproved binary");
    assert!(
        BoundPolicyEvaluator::prepare(
            project.path(),
            &contract,
            &registered,
            bad,
            package,
            store.clone(),
            external.path(),
            now() + 600000
        )
        .is_err()
    );
    let mut cases: Vec<Value> = serde_json::from_slice(
        &fs::read(std::env::var("GUARD_POLICY_FLOWGUARD_CASES").unwrap()).unwrap(),
    )
    .unwrap();
    let facts = &mut cases[0]["input"]["facts"];
    facts["action"] = json!("MERGE");
    facts["subjectDigest"] = json!(bound.binding().subject_digest);
    facts["contractDigest"] = json!(bound.binding().contract_digest);
    facts["authorityContext"]["verifiedSubjectDigest"] = json!(bound.binding().subject_digest);
    facts["authorityContext"]["verifiedContractDigest"] = json!(bound.binding().contract_digest);
    for result in facts["results"].as_array_mut().unwrap() {
        result["subjectDigest"] = json!(bound.binding().subject_digest);
        result["contractDigest"] = json!(bound.binding().contract_digest);
        result["policyDigest"] = json!(bound.binding().policy_digest);
        result["planDigest"] = json!(bound.binding().plan_digest);
    }
    let receipt = bound
        .evaluate(
            &serde_json::to_vec(facts).unwrap(),
            "bound-real",
            Cancellation::new(),
        )
        .await
        .unwrap();
    assert!(
        !receipt.evaluation.incomplete,
        "{:?}; versionObservation={:?}; versionStdout={}; versionStderr={}",
        receipt.evaluation.diagnostics,
        receipt.evaluation.version,
        String::from_utf8_lossy(
            &store
                .read_private(&receipt.evaluation.version.stdout.operation, now())
                .unwrap()
        ),
        String::from_utf8_lossy(
            &store
                .read_private(&receipt.evaluation.version.stderr.operation, now())
                .unwrap()
        )
    );
    assert_eq!(
        receipt.evaluation.binding.contract_digest,
        *contract.digest()
    );
    assert!(receipt.evaluation.execution.started_at.is_some());
    assert_eq!(receipt.evaluation.trust, "UNVERIFIED");
    // intent出现证明前置来源核验已经完成；此后修改源码必须由后置核验撤销结论。
    let observed_store = store.clone();
    let source = project.path().join("src/Example.java");
    let writer = std::thread::spawn(move || {
        let key = format!(
            "guard-policy-operation:{}",
            digest(b"during-policy").as_str()
        );
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
        while observed_store.read_head(&key, now()).unwrap().is_none() {
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        fs::write(source, b"class DuringPolicy {}\n").unwrap();
    });
    let changed = bound
        .evaluate(
            &serde_json::to_vec(facts).unwrap(),
            "during-policy",
            Cancellation::new(),
        )
        .await
        .unwrap();
    writer.join().unwrap();
    assert!(changed.evaluation.incomplete);
    assert!(changed.evaluation.decision.is_none());
    assert!(
        changed.evaluation.execution.started_at.is_some(),
        "{:?}",
        changed.evaluation
    );
    assert!(
        !store
            .read_private(&changed.evaluation.execution.stdout.operation, now())
            .unwrap()
            .is_empty()
    );
    assert!(
        changed
            .evaluation
            .diagnostics
            .iter()
            .any(|line| line.contains("context changed"))
    );
    fs::write(
        project.path().join("src/Example.java"),
        b"class Changed {}\n",
    )
    .unwrap();
    assert!(
        bound
            .evaluate(b"{}", "stale-plan", Cancellation::new())
            .await
            .is_err()
    );
}
