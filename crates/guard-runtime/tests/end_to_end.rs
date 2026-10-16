use guard_contract::{
    ApprovalAttestation, ApprovalAuthority, CapabilityProbe, CompileContext, CompileRequest,
    CompiledContract, ContractLayer, LayerLevel, SourceMaterial, TaskAuthorization,
    VerifierDescriptor, VerifierKind, VerifierObservation, VerifierRegistry, compile,
    snapshot_digest,
};
use guard_plan::{freeze, verifier_digest};
use guard_protocol::{
    Action, Digest, DomainOwner, ExecutionBudget, Extensions, GitOid, GitPath, Identifier,
    ProtocolDocument, SchemaVersion, TaskBinding, TaskScope,
};
use guard_run_ledger::{FinalizeOutcome, RunState};
use guard_runner::{Cancellation, CommandSpec};
use guard_runtime::{DispatchOutcome, NativeVerifierCommand, NativeVerifierCommandV2, Runtime};
use guard_store::{EvidenceStore, StoreLimits};
use guard_subject::{CaptureBinding, CapturedSubject, Selection, capture};
use serde_json::{Value, json};
use sha2::{Digest as Hasher, Sha256};
use std::{collections::BTreeMap, fs, path::Path, process::Command};
use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

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
fn contract(command: &guard_runner::CommandSpec) -> CompiledContract {
    contract_config(
        command,
        NativeVerifierCommand::configuration_digest(command).unwrap(),
    )
}
fn contract_config(command: &guard_runner::CommandSpec, config: Digest) -> CompiledContract {
    let revision = "1";
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
        let mut second = value["obligations"][0].clone();
        second["obligationId"] = json!("OB-2");
        value["obligations"].as_array_mut().unwrap().push(second);

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
        registry.register(VerifierDescriptor { verifier_id: id(if policy { "opa" } else { "java-package-check" }), version: id("1.0.0"), artifact_digest: command.executable_digest.clone(), config_digest: config.clone(), adapter_version: id("1.0.0"), kind: if policy { VerifierKind::PolicyEngine } else { VerifierKind::Domain }, owner: if policy { None } else { Some(DomainOwner::Archguard) }, evidence_types: vec![id(if policy { "policy-decision" } else { "source-report" })], parameter_schema: serde_json::to_vec(&if policy { json!({"type":"object"}) } else { json!({"type":"object","properties":{"maxCycles":{"type":"integer"},"thresholdDecimal":{"type":"string"}},"required":["maxCycles","thresholdDecimal"],"additionalProperties":false}) }).unwrap() }).unwrap();
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
fn captured(root: &Path, contract: &CompiledContract) -> CapturedSubject {
    let base = GitOid::new(git(root, &["rev-parse", "HEAD"])).unwrap();
    capture(
        root,
        &CaptureBinding {
            repository_id: id("repo-example"),
            task_id: id("REQ-1"),
            worktree_id: id("worktree-1"),
            contract_digest: contract.digest().clone(),
            policy_digest: digest(b"policy"),
            verifier_digest: verifier_digest(contract).unwrap(),
            environment_digest: NativeVerifierCommand::environment_digest(&BTreeMap::new())
                .unwrap(),
        },
        &Selection::Worktree { base },
    )
    .unwrap()
}
fn budget() -> ExecutionBudget {
    ExecutionBudget {
        timeout_ms: 15000,
        max_output_bytes: 4 * 1024 * 1024,
        max_concurrency: 1,
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

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}
fn command(root: &Path, mode: &str, marker: &Path) -> CommandSpec {
    let exe = Path::new(env!("CARGO_BIN_EXE_guard-runtime-fixture"))
        .canonicalize()
        .unwrap();
    CommandSpec {
        executable_digest: digest(&fs::read(&exe).unwrap()),
        executable: exe,
        args: vec![mode.into(), marker.to_str().unwrap().into()],
        directory: root.canonicalize().unwrap(),
        environment: BTreeMap::new(),
    }
}
fn native(command: CommandSpec) -> NativeVerifierCommand {
    NativeVerifierCommand {
        verifier_id: id("java-package-check"),
        command,
    }
}

#[tokio::test]
async fn real_process_large_raw_streams_are_separate_and_zero_never_quality_pass() {
    let _fixture_slot = fixture_slot().await;
    let temp = fixture();
    let root = temp.path();
    let storage = tempfile::tempdir().unwrap();
    let marker = storage.path().join("ran");
    let cmd = command(root, "large", &marker);
    let contract = contract(&cmd);
    let subject = captured(root, &contract);
    let bind = binding(&contract);
    let store =
        EvidenceStore::create(&storage.path().join("store"), StoreLimits::default()).unwrap();
    let end = now() + 600000;
    let plan = freeze(root, &contract, &bind, &subject, budget())
        .unwrap()
        .register(&store, end, now())
        .unwrap();
    let store = Arc::new(store);
    let runtime = Runtime::new(&plan, &contract, store.clone(), root, end).unwrap();
    let obligation = plan.plan().verification().obligations[0]
        .obligation_id
        .clone();
    let DispatchOutcome::Finished(result) = runtime
        .execute(&obligation, "run-1", native(cmd), Cancellation::new())
        .await
        .unwrap()
    else {
        panic!("not run")
    };
    assert!(marker.exists());
    assert_eq!(result.exit_code, Some(0));
    assert_eq!(result.state, RunState::Incomplete);
    assert_eq!(result.finalization, Some(FinalizeOutcome::Applied));
    let stdout = store.read_private(&result.stdout.operation, now()).unwrap();
    assert_eq!(stdout.len(), 2300018);
    assert!(stdout.starts_with(b"partial violation\n"));
    assert_eq!(
        store.read_private(&result.stderr.operation, now()).unwrap(),
        b"private stderr\n".to_vec()
    );
    assert!(serde_json::to_vec(&result).unwrap().len() < 8192);
    assert_eq!(result.identity.contract_digest, *contract.digest());
    assert_eq!(
        result.identity.environment_digest,
        plan.capture_binding().environment_digest
    );
    assert!(
        runtime
            .execute(
                &obligation,
                "run-1",
                native(command(root, "large", &marker)),
                Cancellation::new()
            )
            .await
            .is_err()
    );
}

#[tokio::test]
async fn queued_input_drift_must_not_dispatch_a_second_process() {
    let _fixture_slot = fixture_slot().await;
    let temp = fixture();
    let root = temp.path();
    let storage = tempfile::tempdir().unwrap();
    let marker = storage.path().join("ran");
    let cmd = command(root, "wait", &marker);
    let contract = contract(&cmd);
    let subject = captured(root, &contract);
    let bind = binding(&contract);
    let store =
        EvidenceStore::create(&storage.path().join("store"), StoreLimits::default()).unwrap();
    let end = now() + 600000;
    let plan = freeze(
        root,
        &contract,
        &bind,
        &subject,
        ExecutionBudget {
            timeout_ms: 30000,
            ..budget()
        },
    )
    .unwrap()
    .register(&store, end, now())
    .unwrap();
    let store = Arc::new(store);
    let runtime = Runtime::new(&plan, &contract, store.clone(), root, end).unwrap();
    let cancel = Cancellation::new();
    let first_id = id("OB-1");
    let first = runtime.execute(&first_id, "first", native(cmd.clone()), cancel.clone());
    let second = async {
        for _ in 0..3000 {
            if marker.exists() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        assert!(marker.exists());
        let modify = async {
            let scope = guard_run_ledger::RunScope {
                repository_id: contract.context().repository_id.clone(),
                task_id: bind.task_id.clone(),
                obligation_id: id("OB-2"),
                approved_scope_digest: guard_protocol::digest_json(
                    &serde_json::to_vec(&bind.scope).unwrap(),
                )
                .unwrap(),
            };
            let d = digest(b"unrelated inspection identity");
            let identity = guard_run_ledger::RunIdentity {
                binding_digest: d.clone(),
                subject_digest: d.clone(),
                contract_digest: d.clone(),
                policy_digest: d.clone(),
                verifier_digest: d.clone(),
                environment_digest: d,
            };
            let mut started = false;
            for _ in 0..2000 {
                if guard_run_ledger::RunLedger::new(&store)
                    .inspect(&scope, &identity, now())
                    .unwrap()
                    .is_some()
                {
                    started = true;
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
            assert!(
                started,
                "second obligation must really reach the running queue"
            );
            fs::write(root.join("src/Example.java"), b"changed while queued").unwrap();
            cancel.cancel();
        };
        let second_id = id("OB-2");
        let (result, ()) = tokio::join!(
            runtime.execute(
                &second_id,
                "second",
                native(cmd.clone()),
                Cancellation::new()
            ),
            modify
        );
        result
    };
    let (a, b) = tokio::join!(first, second);
    a.unwrap();
    let DispatchOutcome::Finished(result) = b.unwrap() else {
        panic!("not finished")
    };
    assert!(result.input_drifted);
    assert_eq!(result.state, RunState::Incomplete);
    assert_eq!(
        result.execution_status,
        guard_protocol::ExecutionStatus::NotRun
    );
    assert_eq!(result.stop_reason, "InputRejected");
    assert_eq!(
        fs::read(&marker).unwrap(),
        b"x",
        "queued drifted candidate must not execute"
    );
}

#[tokio::test]
async fn wrong_pins_and_stale_inputs_never_dispatch() {
    let _fixture_slot = fixture_slot().await;
    let temp = fixture();
    let root = temp.path();
    let storage = tempfile::tempdir().unwrap();
    let marker = storage.path().join("ran");
    let cmd = command(root, "fail", &marker);
    let contract = contract(&cmd);
    let subject = captured(root, &contract);
    let store = Arc::new(
        EvidenceStore::create(&storage.path().join("store"), StoreLimits::default()).unwrap(),
    );
    let end = now() + 600000;
    let plan = freeze(root, &contract, &binding(&contract), &subject, budget())
        .unwrap()
        .register(&store, end, now())
        .unwrap();
    let runtime = Runtime::new(&plan, &contract, store.clone(), root, end).unwrap();
    let mut wrong = cmd.clone();
    wrong.executable_digest = digest(b"wrong");
    assert!(
        runtime
            .execute(&id("OB-1"), "bad-pin", native(wrong), Cancellation::new())
            .await
            .is_err()
    );
    let mut wrong = cmd.clone();
    wrong.args.push("unapproved".into());
    assert!(
        runtime
            .execute(
                &id("OB-1"),
                "bad-config",
                native(wrong),
                Cancellation::new()
            )
            .await
            .is_err()
    );
    let mut wrong = cmd.clone();
    wrong.environment.insert("UNAPPROVED".into(), "1".into());
    assert!(
        runtime
            .execute(&id("OB-1"), "bad-env", native(wrong), Cancellation::new())
            .await
            .is_err()
    );
    assert!(
        runtime
            .execute(
                &id("unknown"),
                "bad-obligation",
                native(cmd.clone()),
                Cancellation::new()
            )
            .await
            .is_err()
    );
    assert!(!marker.exists());
    fs::write(root.join("src/Example.java"), b"stale").unwrap();
    assert!(
        runtime
            .execute(&id("OB-1"), "stale", native(cmd), Cancellation::new())
            .await
            .is_err()
    );
    assert!(!marker.exists());
}

#[tokio::test]
async fn failed_timed_out_truncated_and_drifted_results_keep_observed_bytes() {
    let _fixture_slot = fixture_slot().await;
    for mode in ["fail", "wait", "truncate", "drift"] {
        let temp = fixture();
        let root = temp.path();
        let storage = tempfile::tempdir().unwrap();
        let marker = storage.path().join("ran");
        let cmd = command(root, mode, &marker);
        let contract = contract(&cmd);
        let subject = captured(root, &contract);
        let store = Arc::new(
            EvidenceStore::create(&storage.path().join("store"), StoreLimits::default()).unwrap(),
        );
        let end = now() + 600000;
        let plan = freeze(root, &contract, &binding(&contract), &subject, budget())
            .unwrap()
            .register(&store, end, now())
            .unwrap();
        let runtime = Runtime::new(&plan, &contract, store.clone(), root, end).unwrap();
        let DispatchOutcome::Finished(result) = runtime
            .execute(&id("OB-1"), "terminal", native(cmd), Cancellation::new())
            .await
            .unwrap()
        else {
            panic!("not finished")
        };
        assert_ne!(result.state, RunState::Pass);
        if mode == "wait" && !marker.exists() {
            assert_eq!(result.state, RunState::Timeout);
            assert!(
                store
                    .read_private(&result.stdout.operation, now())
                    .unwrap()
                    .is_empty()
            );
            assert!(!result.input_drifted);
            continue;
        }
        assert!(
            store
                .read_private(&result.stdout.operation, now())
                .unwrap()
                .starts_with(b"partial violation\n"),
            "mode {mode}: {result:?}"
        );
        match mode {
            "fail" => {
                assert_eq!(result.exit_code, Some(7));
                assert_eq!(result.state, RunState::Fail);
            }
            "wait" => assert_eq!(result.state, RunState::Timeout),
            "truncate" => {
                assert!(result.output_truncated);
                assert_eq!(result.state, RunState::Incomplete);
            }
            "drift" => {
                assert!(result.input_drifted);
                assert_eq!(result.state, RunState::Incomplete);
            }
            _ => unreachable!(),
        }
    }
}

#[tokio::test]
async fn dropped_future_leaves_running_existing_does_not_execute_and_explicit_recovery_advances_sequence()
 {
    let _fixture_slot = fixture_slot().await;
    let temp = fixture();
    let root = temp.path();
    let storage = tempfile::tempdir().unwrap();
    let marker = storage.path().join("ran");
    let cmd = command(root, "wait", &marker);
    let contract = contract(&cmd);
    let subject = captured(root, &contract);
    let store = Arc::new(
        EvidenceStore::create(&storage.path().join("store"), StoreLimits::default()).unwrap(),
    );
    let end = now() + 600000;
    let plan = freeze(root, &contract, &binding(&contract), &subject, budget())
        .unwrap()
        .register(&store, end, now())
        .unwrap();
    let runtime = Runtime::new(&plan, &contract, store.clone(), root, end).unwrap();
    let obligation = id("OB-1");
    let mut running = Box::pin(runtime.execute(
        &obligation,
        "aborted",
        native(cmd.clone()),
        Cancellation::new(),
    ));
    let wait_marker = async {
        for _ in 0..3000 {
            if marker.exists() {
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        panic!("process never started")
    };
    tokio::select! {result=&mut running=>panic!("finished prematurely {result:?}"),()=wait_marker=>{}}
    drop(running);
    let DispatchOutcome::Existing { sequence } = runtime
        .execute(
            &obligation,
            "aborted",
            native(cmd.clone()),
            Cancellation::new(),
        )
        .await
        .unwrap()
    else {
        panic!("not existing")
    };
    assert_eq!(sequence.as_str(), "1");
    assert_eq!(fs::read(&marker).unwrap(), b"x");
    let scope = guard_run_ledger::RunScope {
        repository_id: contract.context().repository_id.clone(),
        task_id: plan.binding().task_id.clone(),
        obligation_id: obligation.clone(),
        approved_scope_digest: guard_protocol::digest_json(
            &serde_json::to_vec(&plan.binding().scope).unwrap(),
        )
        .unwrap(),
    };
    assert!(
        guard_run_ledger::RunLedger::new(&store)
            .recover(&scope, now())
            .unwrap()
    );
    let DispatchOutcome::Finished(result) = runtime
        .execute(
            &obligation,
            "after-recovery",
            native(cmd),
            Cancellation::new(),
        )
        .await
        .unwrap()
    else {
        panic!("not finished")
    };
    assert_eq!(result.sequence.as_str(), "2");
    assert_eq!(result.state, RunState::Timeout);
    assert_eq!(fs::read(&marker).unwrap(), b"xx");
}

#[tokio::test]
async fn cancellation_preserves_partial_bytes() {
    let _fixture_slot = fixture_slot().await;
    let temp = fixture();
    let root = temp.path();
    let storage = tempfile::tempdir().unwrap();
    let marker = storage.path().join("ran");
    let cmd = command(root, "wait", &marker);
    let contract = contract(&cmd);
    let subject = captured(root, &contract);
    let store = Arc::new(
        EvidenceStore::create(&storage.path().join("store"), StoreLimits::default()).unwrap(),
    );
    let end = now() + 600000;
    let plan = freeze(root, &contract, &binding(&contract), &subject, budget())
        .unwrap()
        .register(&store, end, now())
        .unwrap();
    let runtime = Runtime::new(&plan, &contract, store.clone(), root, end).unwrap();
    let cancel = Cancellation::new();
    let obligation = id("OB-1");
    let trigger = async {
        for _ in 0..3000 {
            if marker.exists() {
                tokio::time::sleep(std::time::Duration::from_millis(80)).await;
                cancel.cancel();
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        panic!("no process")
    };
    let (result, ()) = tokio::join!(
        runtime.execute(&obligation, "cancel", native(cmd), cancel.clone()),
        trigger
    );
    let DispatchOutcome::Finished(result) = result.unwrap() else {
        panic!("not finished")
    };
    assert_eq!(result.state, RunState::Cancelled);
    assert!(
        store
            .read_private(&result.stdout.operation, now())
            .unwrap()
            .starts_with(b"partial violation\n")
    );
}

#[tokio::test]
async fn expired_dispatch_budget_is_timeout_without_fabricating_input_drift() {
    let _fixture_slot = fixture_slot().await;
    let temp = fixture();
    let root = temp.path();
    let storage = tempfile::tempdir().unwrap();
    let marker = storage.path().join("ran");
    let cmd = command(root, "large", &marker);
    let contract = contract(&cmd);
    let subject = captured(root, &contract);
    let store = Arc::new(
        EvidenceStore::create(&storage.path().join("store"), StoreLimits::default()).unwrap(),
    );
    let end = now() + 600000;
    let plan = freeze(
        root,
        &contract,
        &binding(&contract),
        &subject,
        ExecutionBudget {
            timeout_ms: 1,
            ..budget()
        },
    )
    .unwrap()
    .register(&store, end, now())
    .unwrap();
    let runtime = Runtime::new(&plan, &contract, store.clone(), root, end).unwrap();
    let DispatchOutcome::Finished(result) = runtime
        .execute(&id("OB-1"), "expired", native(cmd), Cancellation::new())
        .await
        .unwrap()
    else {
        panic!("not finished")
    };
    assert!(!marker.exists());
    assert!(!result.input_drifted);
    assert_eq!(
        result.execution_status,
        guard_protocol::ExecutionStatus::TimedOut
    );
    assert_eq!(result.state, RunState::Timeout);
    assert!(result.recheck_error.as_ref().unwrap().contains("timed out"));
    assert!(
        store
            .read_private(&result.stdout.operation, now())
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn commit_candidate_cannot_execute_dirty_worktree_as_its_frozen_input() {
    let _fixture_slot = fixture_slot().await;
    let temp = fixture();
    let root = temp.path();
    let storage = tempfile::tempdir().unwrap();
    let marker = storage.path().join("ran");
    let cmd = command(root, "large", &marker);
    let contract = contract(&cmd);
    let capture_binding = captured(root, &contract).binding;
    let commit = GitOid::new(git(root, &["rev-parse", "HEAD"])).unwrap();
    let subject = capture(
        root,
        &capture_binding,
        &Selection::Commit {
            commit: commit.clone(),
            base: commit,
        },
    )
    .unwrap();
    let store = Arc::new(
        EvidenceStore::create(&storage.path().join("store"), StoreLimits::default()).unwrap(),
    );
    let end = now() + 600000;
    let plan = freeze(root, &contract, &binding(&contract), &subject, budget())
        .unwrap()
        .register(&store, end, now())
        .unwrap();
    fs::write(
        root.join("src/Example.java"),
        b"DIRTY WORKTREE NOT THE FROZEN COMMIT",
    )
    .unwrap();
    // 对象候选仍为当前已注册对象，不能误把对象复核成功当作工作树执行字节一致。
    plan.assert_current(&store, root, now()).unwrap();
    assert!(
        Runtime::new(&plan, &contract, store, root, end).is_err(),
        "commit must not run directly in the dirty worktree before materialization exists"
    );
    assert!(!marker.exists());
}

#[tokio::test]
async fn v2_all_git_candidates_read_frozen_bytes_in_private_roots_with_stable_approval() {
    let _fixture_slot = fixture_slot().await;
    for kind in ["worktree", "index", "commit", "merge"] {
        let temp = fixture();
        let root = temp.path();
        let storage = tempfile::tempdir().unwrap();
        let scratch = storage.path().join("scratch");
        fs::create_dir(&scratch).unwrap();
        let marker = storage.path().join("ran");
        let merge_base = "class Example {\n int left=0;\n // padding 0\n // padding 1\n // padding 2\n // padding 3\n // padding 4\n // padding 5\n // padding 6\n // padding 7\n int right=0;\n}\n";
        if kind == "merge" {
            fs::write(root.join("src/Example.java"), merge_base).unwrap();
            git(root, &["add", "."]);
            git(root, &["commit", "-m", "merge base"]);
        }
        let base = GitOid::new(git(root, &["rev-parse", "HEAD"])).unwrap();
        let (selection, expected) = match kind {
            "worktree" => {
                fs::write(root.join("src/Example.java"), b"WORKTREE FROZEN").unwrap();
                (
                    Selection::Worktree { base: base.clone() },
                    "WORKTREE FROZEN".to_owned(),
                )
            }
            "index" => {
                fs::write(root.join("src/Example.java"), b"INDEX FROZEN").unwrap();
                git(root, &["add", "src/Example.java"]);
                fs::write(root.join("src/Example.java"), b"DIRTY NOT INDEX").unwrap();
                (
                    Selection::Index { base: base.clone() },
                    "INDEX FROZEN".to_owned(),
                )
            }
            "commit" => {
                fs::write(root.join("src/Example.java"), b"DIRTY NOT COMMIT").unwrap();
                (
                    Selection::Commit {
                        commit: base.clone(),
                        base: base.clone(),
                    },
                    "class Example {}\n".to_owned(),
                )
            }
            "merge" => {
                git(root, &["branch", "target"]);
                git(root, &["checkout", "-b", "head"]);
                fs::write(root.join("head-only.txt"), b"head new object").unwrap();
                fs::write(
                    root.join("src/Example.java"),
                    merge_base.replace("left=0", "left=1"),
                )
                .unwrap();
                git(root, &["add", "."]);
                git(root, &["commit", "-m", "head"]);
                let head = GitOid::new(git(root, &["rev-parse", "HEAD"])).unwrap();
                git(root, &["checkout", "target"]);
                fs::write(
                    root.join("src/Example.java"),
                    merge_base.replace("right=0", "right=2"),
                )
                .unwrap();
                git(root, &["add", "."]);
                git(root, &["commit", "-m", "target"]);
                let target = GitOid::new(git(root, &["rev-parse", "HEAD"])).unwrap();
                fs::write(root.join("src/Example.java"), b"DIRTY NOT MERGE").unwrap();
                (
                    Selection::MergeCandidate {
                        head,
                        target_ref: id("refs/heads/target"),
                        expected_target: target,
                    },
                    merge_base
                        .replace("left=0", "left=1")
                        .replace("right=0", "right=2"),
                )
            }
            _ => unreachable!(),
        };
        let mut cmd = command(root, "read", &marker);
        cmd.args.push("{{subject_root}}/src/Example.java".into());
        cmd.environment.insert(
            "GIT_CEILING_DIRECTORIES".into(),
            "{{subject_parent}}".into(),
        );
        let approved = NativeVerifierCommandV2::configuration_digest(&cmd).unwrap();
        let contract = contract_config(&cmd, approved.clone());
        let mut context = captured(root, &contract).binding;
        context.environment_digest =
            NativeVerifierCommandV2::environment_digest(&cmd.environment).unwrap();
        let subject = capture(root, &context, &selection).unwrap();
        let store = Arc::new(
            EvidenceStore::create(&storage.path().join("store"), StoreLimits::default()).unwrap(),
        );
        let end = now() + 600000;
        let plan = freeze(
            root,
            &contract,
            &binding(&contract),
            &subject,
            ExecutionBudget {
                timeout_ms: 30000,
                ..budget()
            },
        )
        .unwrap()
        .register(&store, end, now())
        .unwrap();
        if kind == "merge" {
            let object = subject
                .manifest
                .iter()
                .find(|entry| entry.path.to_bytes() == b"src/Example.java")
                .unwrap()
                .object_id
                .as_ref()
                .unwrap();
            let found = Command::new("git")
                .arg("-C")
                .arg(root)
                .args(["cat-file", "-e", object.as_str()])
                .output()
                .unwrap();
            assert!(
                !found.status.success(),
                "merged file blob must live only in the private merge objects"
            );
        }
        let runtime =
            Runtime::new_v2(&plan, &contract, store.clone(), root, end, &scratch).unwrap();
        let input = NativeVerifierCommandV2 {
            verifier_id: id("java-package-check"),
            command: cmd.clone(),
        };
        let DispatchOutcome::Finished(result) = runtime
            .execute_v2(&id("OB-1"), "v2-first", input.clone(), Cancellation::new())
            .await
            .unwrap()
        else {
            panic!("not finished")
        };
        let bytes = store.read_private(&result.stdout.operation, now()).unwrap();
        let text = String::from_utf8(bytes).unwrap();
        assert!(
            text.contains(&format!("SOURCE:{expected}")),
            "{kind}: {text} {result:?}"
        );
        assert!(!text.contains("DIRTY NOT"));
        assert_eq!(result.exit_code, Some(0));
        assert!(result.started_at.is_some());
        assert!(result.finished_at.is_some());
        assert!(!result.input_drifted);
        assert_eq!(result.state, RunState::Incomplete);
        assert_eq!(result.invocation.approved_config_digest, approved);
        assert_eq!(result.invocation.subject_digest, subject.identity_digest);
        let cwd = result.invocation.working_directory.as_ref().unwrap();
        assert!(Path::new(cwd).starts_with(&scratch));
        assert_ne!(Path::new(cwd), root.canonicalize().unwrap());
        assert!(result.invocation.materialization.is_some());
        assert!(result.invocation.effective_invocation_digest.is_some());
        assert!(text.contains(&format!("ARG:{cwd}")));
        assert!(text.contains(&format!(
            "CEILING:{}",
            Path::new(cwd).parent().unwrap().display()
        )));
        assert!(
            !Path::new(cwd).exists(),
            "private root cleaned after the completed run"
        );
        if kind == "worktree" {
            let DispatchOutcome::Finished(second) = runtime
                .execute_v2(&id("OB-1"), "v2-second", input, Cancellation::new())
                .await
                .unwrap()
            else {
                panic!("not finished")
            };
            assert_eq!(second.invocation.approved_config_digest, approved);
            assert_eq!(second.plan_digest, result.plan_digest);
            assert_eq!(second.sequence.as_str(), "2");
            assert_ne!(
                second.invocation.working_directory,
                result.invocation.working_directory
            );
        }
        assert_eq!(
            fs::read(root.join("src/Example.java")).unwrap(),
            if kind == "worktree" {
                b"WORKTREE FROZEN".to_vec()
            } else {
                format!("DIRTY NOT {}", kind.to_ascii_uppercase()).into_bytes()
            }
        );
    }
}

async fn v2_mutation_case(mode: &str, mutate_source: bool) {
    let temp = fixture();
    let root = temp.path();
    let storage = tempfile::tempdir().unwrap();
    let marker = storage.path().join("ran");
    let scratch = storage.path().join("scratch");
    fs::create_dir(&scratch).unwrap();
    let mut cmd = command(root, mode, &marker);
    cmd.environment.insert(
        "GIT_CEILING_DIRECTORIES".into(),
        "{{subject_parent}}".into(),
    );
    let contract = contract_config(
        &cmd,
        NativeVerifierCommandV2::configuration_digest(&cmd).unwrap(),
    );
    let mut context = captured(root, &contract).binding;
    context.environment_digest =
        NativeVerifierCommandV2::environment_digest(&cmd.environment).unwrap();
    let base = GitOid::new(git(root, &["rev-parse", "HEAD"])).unwrap();
    let subject = capture(root, &context, &Selection::Worktree { base }).unwrap();
    let store = Arc::new(
        EvidenceStore::create(&storage.path().join("store"), StoreLimits::default()).unwrap(),
    );
    let end = now() + 600000;
    let plan = freeze(
        root,
        &contract,
        &binding(&contract),
        &subject,
        ExecutionBudget {
            timeout_ms: 30000,
            ..budget()
        },
    )
    .unwrap()
    .register(&store, end, now())
    .unwrap();
    let runtime = Runtime::new_v2(&plan, &contract, store.clone(), root, end, &scratch).unwrap();
    let cancel = Cancellation::new();
    let obligation = id("OB-1");
    let execute = runtime.execute_v2(
        &obligation,
        "mutating",
        NativeVerifierCommandV2 {
            verifier_id: id("java-package-check"),
            command: cmd,
        },
        cancel.clone(),
    );
    let result = if mutate_source {
        let ready = storage.path().join("ran.ready");
        let change = async {
            for _ in 0..4000 {
                if ready.exists() {
                    fs::write(root.join("src/Example.java"), b"ORIGINAL SOURCE CHANGED").unwrap();
                    cancel.cancel();
                    return;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
            panic!("source never emitted")
        };
        let (result, ()) = tokio::join!(execute, change);
        result
    } else {
        execute.await
    };
    let DispatchOutcome::Finished(result) = result.unwrap() else {
        panic!("not finished")
    };
    assert!(result.input_drifted, "{mode}: {result:?}");
    assert_eq!(result.state, RunState::Incomplete);
    assert!(result.recheck_error.is_some());
    let text =
        String::from_utf8(store.read_private(&result.stdout.operation, now()).unwrap()).unwrap();
    assert!(text.contains("SOURCE:class Example {}"), "{text}");
    assert!(result.started_at.is_some());
    assert!(result.finished_at.is_some());
    if !mutate_source {
        plan.assert_current(&store, root, now()).unwrap();
        assert_eq!(
            fs::read(root.join("src/Example.java")).unwrap(),
            b"class Example {}\n"
        );
    }
}
#[tokio::test]
async fn v2_original_source_drift_keeps_real_output_and_invalidates_observation() {
    let _fixture_slot = fixture_slot().await;
    v2_mutation_case("read-hold", true).await;
}
#[tokio::test]
async fn v2_materialized_input_tampering_is_incomplete_without_mutating_original() {
    let _fixture_slot = fixture_slot().await;
    v2_mutation_case("tamper", false).await;
}

#[tokio::test]
async fn v2_git_discovery_cannot_escape_to_an_unrelated_scratch_parent_repository() {
    let _fixture_slot = fixture_slot().await;
    let temp = fixture();
    let root = temp.path();
    let storage = tempfile::tempdir().unwrap();
    git(storage.path(), &["init"]);
    let marker = storage.path().join("ran");
    let scratch = storage.path().join("scratch");
    fs::create_dir(&scratch).unwrap();
    let mut cmd = command(root, "git-probe", &marker);
    cmd.args.push("{{subject_root}}".into());
    let executable_name = if cfg!(windows) { "git.exe" } else { "git" };
    let git_path = std::env::split_paths(&std::env::var_os("PATH").unwrap())
        .map(|path| path.join(executable_name))
        .find(|path| path.is_file())
        .unwrap()
        .canonicalize()
        .unwrap();
    cmd.args.push(git_path.to_str().unwrap().into());
    cmd.environment.insert(
        "GIT_CEILING_DIRECTORIES".into(),
        "{{subject_parent}}".into(),
    );
    let contract = contract_config(
        &cmd,
        NativeVerifierCommandV2::configuration_digest(&cmd).unwrap(),
    );
    let mut context = captured(root, &contract).binding;
    context.environment_digest =
        NativeVerifierCommandV2::environment_digest(&cmd.environment).unwrap();
    let base = GitOid::new(git(root, &["rev-parse", "HEAD"])).unwrap();
    let subject = capture(root, &context, &Selection::Worktree { base }).unwrap();
    let store = Arc::new(
        EvidenceStore::create(&storage.path().join("store"), StoreLimits::default()).unwrap(),
    );
    let end = now() + 600000;
    let plan = freeze(
        root,
        &contract,
        &binding(&contract),
        &subject,
        ExecutionBudget {
            timeout_ms: 30000,
            ..budget()
        },
    )
    .unwrap()
    .register(&store, end, now())
    .unwrap();
    let runtime = Runtime::new_v2(&plan, &contract, store.clone(), root, end, &scratch).unwrap();
    let DispatchOutcome::Finished(result) = runtime
        .execute_v2(
            &id("OB-1"),
            "git-ceiling",
            NativeVerifierCommandV2 {
                verifier_id: id("java-package-check"),
                command: cmd,
            },
            Cancellation::new(),
        )
        .await
        .unwrap()
    else {
        panic!("not finished")
    };
    let text =
        String::from_utf8(store.read_private(&result.stdout.operation, now()).unwrap()).unwrap();
    assert!(text.contains("DISCOVERY:false"), "{text} {result:?}");
    assert!(
        text.contains("UNRESTRICTED_DISCOVERY:true"),
        "Git itself must work; ceiling causes the negative result: {text}"
    );
    assert_eq!(result.exit_code, Some(0));
    assert!(!result.input_drifted);
}

// 仅限制跨测试的真实Git/Schema启动负载；每个测试内的并发/单飞仍独立真实执行。
async fn fixture_slot() -> tokio::sync::OwnedSemaphorePermit {
    static SLOTS: std::sync::OnceLock<Arc<tokio::sync::Semaphore>> = std::sync::OnceLock::new();
    SLOTS
        .get_or_init(|| Arc::new(tokio::sync::Semaphore::new(2)))
        .clone()
        .acquire_owned()
        .await
        .unwrap()
}
