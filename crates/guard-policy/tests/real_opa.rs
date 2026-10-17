#![cfg(feature = "real-opa-tests")]
use guard_policy::{OpaConfig, OpaEvaluator, PolicyInput, PolicyPackage};
use guard_protocol::{Digest, ExecutionBudget, Identifier};
use guard_runner::Cancellation;
use guard_store::{EvidenceStore, StoreLimits};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    path::PathBuf,
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
        executable: PathBuf::from(
            std::env::var("GUARD_POLICY_OPA_EXECUTABLE")
                .expect("explicit pinned OPA executable required"),
        ),
        executable_digest: Digest::new(
            std::env::var("GUARD_POLICY_OPA_SHA256")
                .expect("explicit approved binary digest required"),
        )
        .unwrap(),
        version: Identifier::new("1.21.2").unwrap(),
        environment,
        budget: ExecutionBudget {
            timeout_ms: 10000,
            max_output_bytes: 1048576,
            max_concurrency: 1,
        },
    }
}
#[tokio::test]
async fn production_flowguard_matches_independent_cases() {
    let root = PathBuf::from(std::env::var("GUARD_POLICY_FLOWGUARD_PACKAGE").unwrap());
    let manifest = std::fs::read(root.join("manifest.json")).unwrap();
    let package = PolicyPackage::load(&root, &manifest).unwrap();
    let cases = std::fs::read_to_string(std::env::var("GUARD_POLICY_FLOWGUARD_CASES").unwrap())
        .unwrap()
        .replace(
            &format!("sha256:{}", "3".repeat(64)),
            package.digest().as_str(),
        );
    let cases: Vec<Value> = serde_json::from_str(&cases).unwrap();
    assert!(cases.len() >= 26);
    let temp = tempfile::tempdir().unwrap();
    let store = Arc::new(
        EvidenceStore::create(&temp.path().join("store"), StoreLimits::default()).unwrap(),
    );
    let evaluator =
        OpaEvaluator::new(config(), package, store, temp.path(), now() + 600000).unwrap();
    for (index, case) in cases.iter().enumerate() {
        let input = PolicyInput::from_json(&serde_json::to_vec(&case["input"]).unwrap());
        if input.is_err() {
            assert_eq!(case["expectedDecision"], "INCOMPLETE", "{}", case["name"]);
            continue;
        }
        let receipt = evaluator
            .evaluate(
                &input.unwrap(),
                &format!("production-case-{index}"),
                Cancellation::new(),
            )
            .await
            .unwrap();
        assert!(
            !receipt.evaluation.incomplete,
            "{}: {:?}",
            case["name"], receipt.evaluation.diagnostics
        );
        let actual = receipt.evaluation.decision.unwrap();
        assert_eq!(
            serde_json::to_value(actual.decision).unwrap(),
            case["expectedDecision"],
            "{}",
            case["name"]
        );
        assert_eq!(
            actual.missing_obligations.len() as u64,
            case["expectedMissing"].as_u64().unwrap(),
            "{}",
            case["name"]
        );
        assert_eq!(
            actual.active_violations.len() as u64,
            case["expectedFindings"].as_u64().unwrap(),
            "{}",
            case["name"]
        );
        assert_eq!(receipt.evaluation.trust, "UNVERIFIED");
    }
}
fn hash(bytes: &[u8]) -> String {
    use sha2::{Digest as _, Sha256};
    format!("sha256:{:x}", Sha256::digest(bytes))
}
fn fixture(source: &str) -> (tempfile::TempDir, PolicyPackage) {
    let temp = tempfile::tempdir().unwrap();
    let caps = std::fs::read(
        PathBuf::from(std::env::var("GUARD_POLICY_FLOWGUARD_PACKAGE").unwrap())
            .join("capabilities.json"),
    )
    .unwrap();
    std::fs::write(temp.path().join("fixture.rego"), source).unwrap();
    std::fs::write(temp.path().join("capabilities.json"), &caps).unwrap();
    let manifest = serde_json::json!({"schemaVersion":1,"packageId":"test.policy","version":"1","owner":"flowguard","entrypoint":"data.fixture.decision","files":[{"path":"fixture.rego","digest":hash(source.as_bytes())}],"capabilities":{"path":"capabilities.json","digest":hash(&caps)}});
    let package =
        PolicyPackage::load(temp.path(), &serde_json::to_vec(&manifest).unwrap()).unwrap();
    (temp, package)
}
fn make_input(package: &PolicyPackage) -> PolicyInput {
    PolicyInput::from_json(&serde_json::to_vec(&serde_json::json!({"schemaVersion":"guard.policy-input/v1","binding":{"repositoryId":"repo","taskId":"task","subjectDigest":format!("sha256:{}","1".repeat(64)),"contractDigest":format!("sha256:{}","2".repeat(64)),"policyDigest":package.digest(),"planDigest":format!("sha256:{}","4".repeat(64)),"action":"MERGE"},"facts":{}})).unwrap()).unwrap()
}
fn decision(value: &str) -> String {
    format!(
        "package fixture\nimport rego.v1\ndecision := {{\"schemaVersion\":1,\"decision\":\"{value}\",\"reasons\":[],\"requiredActions\":[],\"missingObligations\":[],\"activeViolations\":[]}}\n"
    )
}
#[tokio::test]
async fn actual_opa_errors_undefined_unsafe_capability_and_immutable_source() {
    for (index,source) in ["package fixture\nimport rego.v1\nunrelated := true\n".to_string(),"package fixture\nimport rego.v1\ndecision := 1 / 0\n".into(),"package fixture\nimport rego.v1\ndecision := http.send({\"method\":\"GET\",\"url\":\"http://127.0.0.1:9\"})\n".into(),"package fixture\nTHIS IS INVALID\n".into(),decision("DENY")].iter().enumerate(){
 let (temp,package)=fixture(source);let input=make_input(&package);let store=Arc::new(EvidenceStore::create(&temp.path().join("store"),StoreLimits::default()).unwrap());let evaluator=OpaEvaluator::new(config(),package,store.clone(),temp.path(),now()+600000).unwrap();
 std::fs::write(temp.path().join("fixture.rego"),decision("ALLOW")).unwrap();let receipt=evaluator.evaluate(&input,&format!("negative-{index}"),Cancellation::new()).await.unwrap();assert_eq!(receipt.evaluation.version.status,guard_protocol::ExecutionStatus::Completed,"{:?}",receipt.evaluation.diagnostics);assert!(receipt.evaluation.execution.started_at.is_some());
 if index<4{assert!(receipt.evaluation.incomplete);assert!(receipt.evaluation.decision.is_none());let raw=store.read_private(&receipt.evaluation.execution.stdout.operation,now()).unwrap();assert!(!raw.is_empty());}else{assert!(!receipt.evaluation.incomplete,"{:?}",receipt.evaluation.diagnostics);assert_eq!(receipt.evaluation.decision.unwrap().decision,guard_protocol::Decision::Deny);}
 assert!(evaluator.evaluate(&input,&format!("negative-{index}"),Cancellation::new()).await.is_err(),"single operation cannot silently replay");
 }
}
#[tokio::test]
async fn interrupted_intent_survives_future_drop_and_reopen() {
    let (temp, package) = fixture(&decision("ALLOW"));
    let input = make_input(&package);
    let store = Arc::new(
        EvidenceStore::create(&temp.path().join("store"), StoreLimits::default()).unwrap(),
    );
    let evaluator = OpaEvaluator::new(
        config(),
        package,
        store.clone(),
        temp.path(),
        now() + 600000,
    )
    .unwrap();
    let task_evaluator = evaluator.clone();
    let task_input = input.clone();
    let task = tokio::spawn(async move {
        task_evaluator
            .evaluate(&task_input, "interrupted", Cancellation::new())
            .await
    });
    let key = format!("guard-policy-operation:{}", hash(b"interrupted"));
    let limit = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while store.read_head(&key, now()).unwrap().is_none() {
        assert!(std::time::Instant::now() < limit);
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
    task.abort();
    let _ = task.await;
    let reopened = EvidenceStore::open(&temp.path().join("store"), StoreLimits::default()).unwrap();
    assert!(reopened.read_head(&key, now()).unwrap().is_some());
    assert!(
        evaluator
            .evaluate(&input, "interrupted", Cancellation::new())
            .await
            .is_err()
    );
}
#[tokio::test]
async fn actual_timeout_cancel_and_output_truncation_never_form_a_decision() {
    let expensive = "package fixture\nimport rego.v1\ndecision := count([crypto.sha256(sprintf(\"%d-%d\",[a,b])) | a := numbers.range(0,3000)[_]; b := numbers.range(0,3000)[_]])\n";
    let (temp, package) = fixture(expensive);
    let input = make_input(&package);
    let store = Arc::new(
        EvidenceStore::create(&temp.path().join("store"), StoreLimits::default()).unwrap(),
    );
    let mut cfg = config();
    cfg.budget.timeout_ms = 6000;
    let evaluator = OpaEvaluator::new(cfg, package, store, temp.path(), now() + 600000).unwrap();
    let receipt = evaluator
        .evaluate(&input, "real-timeout", Cancellation::new())
        .await
        .unwrap();
    assert_eq!(
        receipt.evaluation.version.status,
        guard_protocol::ExecutionStatus::Completed
    );
    assert!(
        receipt.evaluation.execution.started_at.is_some(),
        "timeout test must actually dispatch OPA"
    );
    assert_eq!(
        receipt.evaluation.execution.status,
        guard_protocol::ExecutionStatus::TimedOut
    );
    assert!(receipt.evaluation.incomplete);
    assert!(receipt.evaluation.decision.is_none());
    let cancel = Cancellation::new();
    cancel.cancel();
    let cancelled = evaluator
        .evaluate(&input, "pre-cancel", cancel)
        .await
        .unwrap();
    assert!(cancelled.evaluation.incomplete);
    assert!(cancelled.evaluation.execution.started_at.is_none());
    let (other, package) = fixture(&format!(
        "package fixture\nimport rego.v1\ndecision := [\"{}\"]\n",
        "x".repeat(6000)
    ));
    let input = make_input(&package);
    let store = Arc::new(
        EvidenceStore::create(&other.path().join("store"), StoreLimits::default()).unwrap(),
    );
    let mut cfg = config();
    cfg.budget.max_output_bytes = 1024;
    let evaluator = OpaEvaluator::new(cfg, package, store, other.path(), now() + 600000).unwrap();
    let receipt = evaluator
        .evaluate(&input, "truncated", Cancellation::new())
        .await
        .unwrap();
    assert!(receipt.evaluation.execution.output_truncated);
    assert!(receipt.evaluation.incomplete);
    assert!(receipt.evaluation.decision.is_none());
}
#[test]
fn preparation_queue_cannot_restart_the_policy_budget_or_spawn_opa() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .max_blocking_threads(1)
        .enable_all()
        .build()
        .unwrap();
    let (temp, package) = fixture(&decision("ALLOW"));
    let input = make_input(&package);
    let store = Arc::new(
        EvidenceStore::create(&temp.path().join("store"), StoreLimits::default()).unwrap(),
    );
    let mut cfg = config();
    cfg.budget.timeout_ms = 250;
    let evaluator = OpaEvaluator::new(cfg, package, store, temp.path(), now() + 600000).unwrap();
    let (tx, rx) = std::sync::mpsc::channel();
    let held = runtime.spawn_blocking(move || {
        tx.send(()).unwrap();
        std::thread::sleep(std::time::Duration::from_secs(3));
    });
    rx.recv().unwrap();
    let started = std::time::Instant::now();
    let receipt = runtime
        .block_on(evaluator.evaluate(&input, "queue-deadline", Cancellation::new()))
        .unwrap();
    assert!(
        started.elapsed() < std::time::Duration::from_secs(2),
        "preparation must not receive a fresh independent budget"
    );
    assert!(receipt.evaluation.incomplete);
    assert!(receipt.evaluation.decision.is_none());
    assert!(receipt.evaluation.version.started_at.is_none());
    assert!(receipt.evaluation.execution.started_at.is_none());
    assert_eq!(
        receipt.evaluation.version.status,
        guard_protocol::ExecutionStatus::TimedOut
    );
    runtime.block_on(held).unwrap();
}
