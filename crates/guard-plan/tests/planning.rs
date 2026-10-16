use guard_contract::{
    ApprovalAttestation, ApprovalAuthority, CapabilityProbe, CompileContext, CompileRequest,
    CompiledContract, ContractLayer, LayerLevel, SourceMaterial, TaskAuthorization,
    VerifierDescriptor, VerifierKind, VerifierObservation, VerifierRegistry, compile,
    snapshot_digest,
};
use guard_plan::{
    MigrationAttestation, MigrationAuthority, MigrationRequest, PlanStatus, freeze, load, migrate,
    restore_current, verifier_digest,
};
use guard_protocol::{
    Action, Digest, DomainOwner, ExecutionBudget, Extensions, GitOid, GitPath, Identifier,
    ProtocolDocument, SchemaVersion, TaskBinding, TaskScope,
};
use guard_store::{EvidenceStore, StoreLimits};
use guard_subject::{CaptureBinding, CapturedSubject, Selection, capture};
use serde_json::{Value, json};
use sha2::{Digest as Hasher, Sha256};
use std::{collections::BTreeMap, fs, path::Path, process::Command};

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
fn contract(bytes: &[u8], revision: &str) -> CompiledContract {
    contract_with_protection(bytes, revision, false)
}
fn contract_with_protection(bytes: &[u8], revision: &str, protected: bool) -> CompiledContract {
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
        registry.register(VerifierDescriptor { verifier_id: id(if policy { "opa" } else { "java-package-check" }), version: id("1.0.0"), artifact_digest: digest(b"binary"), config_digest: digest(b"config"), adapter_version: id("1.0.0"), kind: if policy { VerifierKind::PolicyEngine } else { VerifierKind::Domain }, owner: if policy { None } else { Some(DomainOwner::Archguard) }, evidence_types: vec![id(if policy { "policy-decision" } else { "source-report" })], parameter_schema: serde_json::to_vec(&if policy { json!({"type":"object"}) } else { json!({"type":"object","properties":{"maxCycles":{"type":"integer"},"thresholdDecimal":{"type":"string"}},"required":["maxCycles","thresholdDecimal"],"additionalProperties":false}) }).unwrap() }).unwrap();
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
            environment_digest: digest(b"jdk17"),
        },
        &Selection::Worktree { base },
    )
    .unwrap()
}
fn budget() -> ExecutionBudget {
    ExecutionBudget {
        timeout_ms: 2000,
        max_output_bytes: 100000,
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

#[test]
fn real_git_plan_is_frozen_before_results_and_sqlite_round_trip_is_exact() {
    let temp = fixture();
    let root = temp.path();
    let contract = contract(b"class Example {}\n", "1");
    let binding = binding(&contract);
    let subject = captured(root, &contract);
    let plan = freeze(root, &contract, &binding, &subject, budget()).unwrap();
    assert_eq!(plan.status(), PlanStatus::Ready);
    assert_eq!(
        plan.plan().unwrap().verification().obligations.len(),
        contract.obligations().len()
    );
    let external = tempfile::tempdir().unwrap();
    let store =
        EvidenceStore::create(&external.path().join("store"), StoreLimits::default()).unwrap();
    plan.persist(&store, "frozen-REQ-1", 5000, 1000).unwrap();
    let active = plan.register(&store, 5000, 1000).unwrap();
    active.assert_current(&store, root, 1001).unwrap();
    let loaded = load(
        &store,
        "frozen-REQ-1",
        1001,
        root,
        &contract,
        &binding,
        &subject,
        budget(),
    )
    .unwrap();
    assert_eq!(plan.digest(), loaded.digest());
    assert_eq!(
        active.head(),
        restore_current(
            &store,
            "frozen-REQ-1",
            1001,
            root,
            &contract,
            &binding,
            &subject,
            budget()
        )
        .unwrap()
        .head()
    );
    let mut mutated = serde_json::to_value(plan.plan().unwrap().verification()).unwrap();
    mutated["obligations"] = json!([]);
    assert_ne!(
        mutated,
        serde_json::to_value(loaded.plan().unwrap().verification()).unwrap()
    );
}

#[test]
fn candidate_rules_do_not_replace_approved_contract_and_missing_input_is_incomplete() {
    let temp = fixture();
    let root = temp.path();
    let contract = contract(b"class Example {}\n", "1");
    let binding = binding(&contract);
    fs::write(root.join("architecture.json"), b"candidate weakened rules").unwrap();
    let subject = captured(root, &contract);
    let plan = freeze(root, &contract, &binding, &subject, budget()).unwrap();
    assert_eq!(
        plan.plan().unwrap().verification().contract_digest,
        *contract.digest()
    );
    assert_eq!(plan.plan().unwrap().verification().obligations.len(), 1);
    fs::remove_file(root.join("src/Example.java")).unwrap();
    let missing = freeze(
        root,
        &contract,
        &binding,
        &captured(root, &contract),
        budget(),
    )
    .unwrap();
    assert_eq!(missing.status(), PlanStatus::Incomplete);
    assert_eq!(missing.issues().len(), 1);
    assert_eq!(missing.contract_obligations().len(), 1);
    assert!(missing.plan().is_none());
}

#[test]
fn forged_or_stale_capture_binding_scope_and_tool_pins_are_rejected() {
    let temp = fixture();
    let root = temp.path();
    let contract = contract(b"class Example {}\n", "1");
    let binding = binding(&contract);
    let subject = captured(root, &contract);
    let mut forged = subject.clone();
    let ProtocolDocument::SubjectSnapshot(release) = guard_protocol::decode(include_bytes!(
        "../../guard-protocol/fixtures/valid/subject-release-artifact.json"
    ))
    .unwrap()
    .document()
    .clone() else {
        panic!("release fixture")
    };
    forged.snapshot = release;
    assert!(
        freeze(root, &contract, &binding, &forged, budget())
            .unwrap_err()
            .contains("Git-only planner")
    );
    let mut forged = subject.clone();
    forged.manifest.clear();
    assert!(freeze(root, &contract, &binding, &forged, budget()).is_err());
    let mut forged = subject.clone();
    forged.binding.verifier_digest = digest(b"fake");
    assert!(freeze(root, &contract, &binding, &forged, budget()).is_err());
    let mut wrong = binding.clone();
    wrong.worktree_id = id("other-worktree");
    assert!(freeze(root, &contract, &wrong, &subject, budget()).is_err());
    let mut wrong = binding.clone();
    wrong.scope.actions.push(Action::Push);
    assert!(freeze(root, &contract, &wrong, &subject, budget()).is_err());
    fs::write(root.join("src/Example.java"), b"changed").unwrap();
    assert!(freeze(root, &contract, &binding, &subject, budget()).is_err());
}

#[test]
fn ordinary_candidate_changes_keep_approval_but_bind_fresh_inputs_and_protected_changes_do_not() {
    let temp = fixture();
    let root = temp.path();
    let baseline = contract(b"class Example {}\n", "1");
    let binding = binding(&baseline);
    let original = freeze(
        root,
        &baseline,
        &binding,
        &captured(root, &baseline),
        budget(),
    )
    .unwrap();
    fs::write(
        root.join("src/Example.java"),
        b"class Example { int value; }\n",
    )
    .unwrap();
    let changed = freeze(
        root,
        &baseline,
        &binding,
        &captured(root, &baseline),
        budget(),
    )
    .unwrap();
    assert_eq!(changed.status(), PlanStatus::Ready);
    assert_eq!(
        changed.binding().contract_digest,
        original.binding().contract_digest
    );
    assert_eq!(
        changed.binding().authorization_refs,
        original.binding().authorization_refs
    );
    assert!(
        !changed
            .plan()
            .unwrap()
            .equivalent_inputs(original.plan().unwrap())
    );
    assert_ne!(
        changed.plan().unwrap().verification().targets[0].input_digest,
        original.plan().unwrap().verification().targets[0].input_digest
    );
    let protected = contract_with_protection(b"class Example {}\n", "1", true);
    let mismatch = freeze(
        root,
        &protected,
        &self::binding(&protected),
        &captured(root, &protected),
        budget(),
    )
    .unwrap();
    assert_eq!(mismatch.status(), PlanStatus::Incomplete);
    assert!(mismatch.plan().is_none());
    assert_eq!(mismatch.issues()[0].code, "protected_input_changed");
    assert!(
        mismatch.contract_obligations()[0].targets[0]
            .protected_digest
            .is_some()
    );
}

struct MigrationDenied;
impl MigrationAuthority for MigrationDenied {
    fn verify(&self, _request: &MigrationRequest) -> Result<MigrationAttestation, String> {
        Err("revoked or not approved".into())
    }
}
struct WrongMigration;
impl MigrationAuthority for WrongMigration {
    fn verify(&self, _request: &MigrationRequest) -> Result<MigrationAttestation, String> {
        Ok(MigrationAttestation {
            authority_id: id("fixture-authority"),
            approval_ref: id("wrong"),
            request_digest: digest(b"other-request"),
        })
    }
}

#[test]
fn wrong_or_revoked_migration_and_other_task_do_not_adopt_new_contract() {
    let temp = fixture();
    let root = temp.path();
    let old = contract(b"class Example {}\n", "1");
    let external = tempfile::tempdir().unwrap();
    let store =
        EvidenceStore::create(&external.path().join("store"), StoreLimits::default()).unwrap();
    let original = freeze(root, &old, &binding(&old), &captured(root, &old), budget())
        .unwrap()
        .register(&store, 5000, 1000)
        .unwrap();
    let revised = contract(b"class Example {}\n", "2");
    let new_binding = binding(&revised);
    let subject = captured(root, &revised);
    for authority in [&MigrationDenied as &dyn MigrationAuthority, &WrongMigration] {
        assert!(
            migrate(
                &original,
                &store,
                5000,
                1001,
                root,
                &revised,
                &new_binding,
                &subject,
                budget(),
                Some(authority)
            )
            .is_err()
        );
    }
    let mut cross_task = new_binding.clone();
    cross_task.task_id = id("another-task");
    assert!(
        migrate(
            &original,
            &store,
            5000,
            1001,
            root,
            &revised,
            &cross_task,
            &subject,
            budget(),
            Some(&MigrationApproved)
        )
        .is_err()
    );
    assert_eq!(original.binding().contract_digest, *old.digest());
}

#[test]
fn persisted_json_or_changed_budget_cannot_reconstruct_another_plan() {
    let temp = fixture();
    let root = temp.path();
    let contract = contract(b"class Example {}\n", "1");
    let binding = binding(&contract);
    let subject = captured(root, &contract);
    let original = freeze(root, &contract, &binding, &subject, budget()).unwrap();
    let external = tempfile::tempdir().unwrap();
    let store =
        EvidenceStore::create(&external.path().join("store"), StoreLimits::default()).unwrap();
    original.persist(&store, "original", 5000, 1000).unwrap();
    let mut modified_budget = budget();
    modified_budget.timeout_ms = 1000;
    assert!(
        load(
            &store,
            "original",
            1001,
            root,
            &contract,
            &binding,
            &subject,
            modified_budget
        )
        .is_err()
    );
    let mut raw: Value =
        serde_json::from_slice(&store.read_private("original", 1001).unwrap()).unwrap();
    raw["verificationPlan"]["obligations"] = json!([]);
    store
        .put(
            "attacker-repacked",
            &serde_json::to_vec(&raw).unwrap(),
            5000,
            1000,
        )
        .unwrap();
    assert!(
        load(
            &store,
            "attacker-repacked",
            1001,
            root,
            &contract,
            &binding,
            &subject,
            budget()
        )
        .is_err()
    );
}

struct MigrationApproved;
impl MigrationAuthority for MigrationApproved {
    fn verify(&self, request: &MigrationRequest) -> Result<MigrationAttestation, String> {
        Ok(MigrationAttestation {
            authority_id: id("independent-migration-authority"),
            approval_ref: id("MIGRATE-1"),
            request_digest: request.digest().clone(),
        })
    }
}
#[test]
fn contract_revision_requires_explicit_independent_migration_and_invalidates_old_evidence() {
    let temp = fixture();
    let root = temp.path();
    let old = contract(b"class Example {}\n", "1");
    let old_binding = binding(&old);
    let old_subject = captured(root, &old);
    let external = tempfile::tempdir().unwrap();
    let store =
        EvidenceStore::create(&external.path().join("store"), StoreLimits::default()).unwrap();
    let draft = freeze(root, &old, &old_binding, &old_subject, budget()).unwrap();
    draft.persist(&store, "old-plan", 5000, 1000).unwrap();
    let frozen = draft.register(&store, 5000, 1000).unwrap();
    let revised = contract(b"class Example {}\n", "2");
    let revised_binding = binding(&revised);
    let revised_subject = captured(root, &revised);
    assert_eq!(frozen.plan().verification().contract_digest, *old.digest());
    assert!(
        freeze(root, &revised, &revised_binding, &revised_subject, budget())
            .unwrap()
            .register(&store, 5000, 1001)
            .is_err()
    );
    assert!(
        migrate(
            &frozen,
            &store,
            5000,
            1001,
            root,
            &revised,
            &revised_binding,
            &revised_subject,
            budget(),
            None
        )
        .is_err()
    );
    let migrated = migrate(
        &frozen,
        &store,
        5000,
        1001,
        root,
        &revised,
        &revised_binding,
        &revised_subject,
        budget(),
        Some(&MigrationApproved),
    )
    .unwrap();
    assert!(migrated.previous_evidence_requires_revalidation());
    assert_eq!(
        migrated.plan().plan().verification().contract_digest,
        *revised.digest()
    );
    assert_eq!(frozen.plan().verification().contract_digest, *old.digest());
    assert!(frozen.assert_current(&store, root, 1002).is_err());
    assert!(
        restore_current(
            &store,
            "old-plan",
            1002,
            root,
            &old,
            &old_binding,
            &old_subject,
            budget()
        )
        .is_err()
    );
    migrated.plan().assert_current(&store, root, 1002).unwrap();
    assert!(migrated.plan().head().revision > frozen.head().revision);
}

#[test]
fn normal_candidate_refresh_reuses_active_baseline_but_stale_plan_is_rejected() {
    let temp = fixture();
    let root = temp.path();
    let contract = contract(b"class Example {}\n", "1");
    let binding = binding(&contract);
    let external = tempfile::tempdir().unwrap();
    let store =
        EvidenceStore::create(&external.path().join("store"), StoreLimits::default()).unwrap();
    let old = freeze(
        root,
        &contract,
        &binding,
        &captured(root, &contract),
        budget(),
    )
    .unwrap()
    .register(&store, 5000, 1000)
    .unwrap();
    fs::write(
        root.join("src/Example.java"),
        b"class Example { int value; }\n",
    )
    .unwrap();
    let new = freeze(
        root,
        &contract,
        &binding,
        &captured(root, &contract),
        budget(),
    )
    .unwrap()
    .register(&store, 6000, 1001)
    .unwrap();
    assert_eq!(old.head(), new.head());
    assert!(old.assert_current(&store, root, 1002).is_err());
    new.assert_current(&store, root, 1002).unwrap();
    assert_ne!(old.plan().digest(), new.plan().digest());
}

#[test]
fn changing_only_session_does_not_create_a_second_task_head() {
    let temp = fixture();
    let root = temp.path();
    let contract = contract(b"class Example {}\n", "1");
    let binding = binding(&contract);
    let subject = captured(root, &contract);
    let external = tempfile::tempdir().unwrap();
    let store =
        EvidenceStore::create(&external.path().join("store"), StoreLimits::default()).unwrap();
    freeze(root, &contract, &binding, &subject, budget())
        .unwrap()
        .register(&store, 5000, 1000)
        .unwrap();
    let mut session = binding.clone();
    session.session_id = id("another-session");
    assert!(
        freeze(root, &contract, &session, &subject, budget())
            .unwrap()
            .register(&store, 5000, 1001)
            .is_err()
    );
}

#[test]
fn self_declared_worktree_id_cannot_bypass_active_task_migration() {
    let temp = fixture();
    let root = temp.path();
    let old = contract(b"class Example {}\n", "1");
    let external = tempfile::tempdir().unwrap();
    let store =
        EvidenceStore::create(&external.path().join("store"), StoreLimits::default()).unwrap();
    let active = freeze(root, &old, &binding(&old), &captured(root, &old), budget())
        .unwrap()
        .register(&store, 5000, 1000)
        .unwrap();
    let revised = contract(b"class Example {}\n", "2");
    let mut new_binding = binding(&revised);
    new_binding.worktree_id = id("self-declared-new-worktree");
    let existing = captured(root, &revised);
    let mut capture_binding = existing.binding;
    capture_binding.worktree_id = new_binding.worktree_id.clone();
    let subject = capture(root, &capture_binding, &existing.selection).unwrap();
    let draft = freeze(root, &revised, &new_binding, &subject, budget()).unwrap();
    assert!(
        draft.register(&store, 5000, 1001).is_err(),
        "new worktree ID created another active head for same task"
    );
    active.assert_current(&store, root, 1001).unwrap();
    let migrated = migrate(
        &active,
        &store,
        5000,
        1001,
        root,
        &revised,
        &new_binding,
        &subject,
        budget(),
        Some(&MigrationApproved),
    )
    .unwrap();
    migrated.plan().assert_current(&store, root, 1002).unwrap();
    assert!(active.assert_current(&store, root, 1002).is_err());
}

fn registry_key() -> String {
    let digest = guard_protocol::digest_json(&serde_json::to_vec(&json!({"schemaVersion":"guard.task-head/v1","repository":id("repo-example"),"task":id("REQ-1")})).unwrap()).unwrap();
    format!("guard-plan:task:{}", digest.as_str())
}

struct CompetingMigration<'a>(&'a EvidenceStore);
impl MigrationAuthority for CompetingMigration<'_> {
    fn verify(&self, request: &MigrationRequest) -> Result<MigrationAttestation, String> {
        self.0.put(
            "competing-update",
            b"another controller completed first",
            5000,
            1001,
        )?;
        assert!(matches!(
            self.0.compare_and_swap_head(
                &registry_key(),
                Some(request.previous_head()),
                "competing-update",
                1001
            )?,
            guard_store::HeadUpdate::Applied(_)
        ));
        Ok(MigrationAttestation {
            authority_id: id("fixture-controller"),
            approval_ref: id("MIGRATE-race"),
            request_digest: request.digest().clone(),
        })
    }
}

#[test]
fn current_head_changed_while_waiting_for_approval_cannot_be_overwritten() {
    let temp = fixture();
    let root = temp.path();
    let old = contract(b"class Example {}\n", "1");
    let external = tempfile::tempdir().unwrap();
    let store =
        EvidenceStore::create(&external.path().join("store"), StoreLimits::default()).unwrap();
    let original = freeze(root, &old, &binding(&old), &captured(root, &old), budget())
        .unwrap()
        .register(&store, 5000, 1000)
        .unwrap();
    let revised = contract(b"class Example {}\n", "2");
    assert!(
        migrate(
            &original,
            &store,
            5000,
            1001,
            root,
            &revised,
            &binding(&revised),
            &captured(root, &revised),
            budget(),
            Some(&CompetingMigration(&store))
        )
        .is_err()
    );
    assert_eq!(
        store
            .read_head(&registry_key(), 1002)
            .unwrap()
            .unwrap()
            .bytes,
        b"another controller completed first"
    );
    assert!(original.assert_current(&store, root, 1002).is_err());
}

#[test]
fn independently_approved_first_bindings_compete_for_one_real_sqlite_head() {
    let temp = fixture();
    let root = temp.path();
    let external = tempfile::tempdir().unwrap();
    let store_path = external.path().join("store");
    EvidenceStore::create(&store_path, StoreLimits::default()).unwrap();
    let a = contract(b"class Example {}\n", "1");
    let b = contract(b"class Example {}\n", "2");
    let drafts = [
        freeze(root, &a, &binding(&a), &captured(root, &a), budget()).unwrap(),
        freeze(root, &b, &binding(&b), &captured(root, &b), budget()).unwrap(),
    ];
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let handles: Vec<_> = drafts
        .into_iter()
        .map(|draft| {
            let path = store_path.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                let store = EvidenceStore::open(&path, StoreLimits::default()).unwrap();
                draft.register(&store, 5000, 1000)
            })
        })
        .collect();
    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    let store = EvidenceStore::open(&store_path, StoreLimits::default()).unwrap();
    results
        .iter()
        .find_map(|r| r.as_ref().ok())
        .unwrap()
        .assert_current(&store, root, 1001)
        .unwrap();
}
