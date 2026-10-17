use base64::{Engine, engine::general_purpose::STANDARD};
use guard_authority::{
    AnchorSource, ApprovalMaterial, ApprovalPermission, ApprovalPurpose, ApprovalVerifier,
    ApprovedScope, AuthorityAnchor, AuthorityClock, PrincipalRole, ReceiptApprovalAuthority,
    ReceiptBundle, ReceiptSource, RevocationSnapshot, RevocationSource, signing_bytes,
};
use guard_contract::{
    ApprovalAttestation, ApprovalAuthority, CapabilityProbe, CompileContext, CompileRequest,
    CompiledContract, ContractLayer, LayerLevel, SourceMaterial, TaskAuthorization,
    VerifierDescriptor, VerifierKind, VerifierObservation, VerifierRegistry, compile,
    snapshot_digest,
};
use guard_plan::{
    MigrationAttestation, MigrationAuthority, MigrationRequest, freeze, migrate, verifier_digest,
};
use guard_protocol::{
    Action, Digest, DomainOwner, ExecutionBudget, Extensions, GitOid, GitPath, Identifier,
    ProtocolDocument, SchemaVersion, TaskBinding, TaskScope,
};
use guard_protocol::{ApprovalIntent, UtcTimestamp};
use guard_store::{EvidenceStore, StoreLimits};
use guard_subject::{CaptureBinding, CapturedSubject, Selection, capture};
use ring::signature::{Ed25519KeyPair, KeyPair};
use serde_json::{Value, json};
use sha2::{Digest as Hasher, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::Path,
    process::Command,
    sync::{Arc, Mutex},
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

struct Ports {
    receipts: Mutex<BTreeMap<Identifier, ReceiptBundle>>,
    revoked: Mutex<bool>,
}
impl Ports {
    fn new() -> Self {
        Self {
            receipts: Mutex::new(BTreeMap::new()),
            revoked: Mutex::new(false),
        }
    }
    fn approve(
        &self,
        reference: &Identifier,
        digest: &Digest,
        scope: &ApprovedScope,
        purpose: ApprovalPurpose,
    ) {
        let material = ApprovalMaterial {
            schema_version: "guard.approval-material/v1".into(),
            audience: id("controller-1"),
            requested_by: id("coding-agent"),
            action: Action::Merge,
            purpose,
            scope: scope.clone(),
        };
        let bytes = serde_json::to_vec(&material).unwrap();
        let decoded = guard_protocol::decode(include_bytes!(
            "../../guard-protocol/fixtures/valid/approval-receipt.json"
        ))
        .unwrap();
        let ProtocolDocument::ApprovalReceipt(mut receipt) = decoded.document().clone() else {
            panic!("fixture")
        };
        receipt.receipt_id = reference.clone();
        receipt.intent = ApprovalIntent::RequirementScope;
        receipt.scope_digest = scope.digest().unwrap();
        receipt.contract_digest = digest.clone();
        receipt.subject_binding.kind = receipt.intent;
        receipt.subject_binding.scope_digest = receipt.scope_digest.clone();
        receipt.subject_binding.subject_digest = None;
        receipt.proof.provenance_digest = guard_protocol::digest_json(&bytes).unwrap();
        let pair = Ed25519KeyPair::from_seed_unchecked(&[37; 32]).unwrap();
        receipt.proof.signature =
            id(&STANDARD.encode(pair.sign(&signing_bytes(&receipt).unwrap()).as_ref()));
        self.receipts.lock().unwrap().insert(
            reference.clone(),
            ReceiptBundle {
                receipt: serde_json::to_vec(&ProtocolDocument::ApprovalReceipt(receipt)).unwrap(),
                material: bytes,
            },
        );
    }
}
impl AnchorSource for Ports {
    fn current(&self, key: &Identifier) -> Result<AuthorityAnchor, String> {
        if key != &id("external-key-1") {
            return Err("unknown independent key".into());
        }
        let pair = Ed25519KeyPair::from_seed_unchecked(&[37; 32]).unwrap();
        Ok(AuthorityAnchor {
            key_id: key.clone(),
            revocation_authority: id("protected-revocation"),
            issuer: id("external-issuer"),
            public_key: pair.public_key().as_ref().try_into().unwrap(),
            identities: ["reviewer-1", "reviewer-2"]
                .into_iter()
                .map(|principal| PrincipalRole {
                    principal: id(principal),
                    role: id("architect"),
                    purposes: vec![
                        ApprovalPermission::OrganizationLayer,
                        ApprovalPermission::ProjectLayer,
                        ApprovalPermission::TaskLayer,
                        ApprovalPermission::Task,
                        ApprovalPermission::Migration,
                    ],
                    intents: vec![ApprovalIntent::RequirementScope],
                })
                .collect(),
            audiences: vec![id("controller-1")],
            intents: vec![ApprovalIntent::RequirementScope],
        })
    }
}
impl ReceiptSource for Ports {
    fn get(&self, reference: &Identifier) -> Result<ReceiptBundle, String> {
        self.receipts
            .lock()
            .unwrap()
            .get(reference)
            .cloned()
            .ok_or("not independently approved".into())
    }
}
impl AuthorityClock for Ports {
    fn now(&self) -> Result<UtcTimestamp, String> {
        UtcTimestamp::new("2026-10-17T00:01:00Z")
    }
}
impl RevocationSource for Ports {
    fn current(
        &self,
        issuer: &Identifier,
        key: &Identifier,
        receipt_id: &Identifier,
        reference: &Identifier,
    ) -> Result<RevocationSnapshot, String> {
        let revoked = *self.revoked.lock().unwrap();
        Ok(RevocationSnapshot {
            authority: id("protected-revocation"),
            issuer: issuer.clone(),
            key_id: key.clone(),
            receipt_id: receipt_id.clone(),
            revocation_ref: reference.clone(),
            revision: if revoked { 2 } else { 1 },
            state_digest: digest(if revoked { b"revoked" } else { b"active" }),
            valid_from: UtcTimestamp::new("2026-10-17T00:00:00Z").unwrap(),
            expires_at: UtcTimestamp::new("2026-10-17T00:10:00Z").unwrap(),
            revoked,
        })
    }
}
struct RecordTask<'a> {
    authority: &'a ReceiptApprovalAuthority,
    digest: Mutex<Option<Digest>>,
}
impl ApprovalAuthority for RecordTask<'_> {
    fn verify_layer(
        &self,
        context: &CompileContext,
        level: LayerLevel,
        reference: &Identifier,
        digest: &Digest,
        action: Action,
    ) -> Result<ApprovalAttestation, String> {
        self.authority
            .verify_layer(context, level, reference, digest, action)
    }
    fn authorize_task(
        &self,
        context: &CompileContext,
        digest: &Digest,
        action: Action,
    ) -> Result<TaskAuthorization, String> {
        *self.digest.lock().unwrap() = Some(digest.clone());
        self.authority.authorize_task(context, digest, action)
    }
}
struct RecordMigration<'a> {
    authority: &'a ReceiptApprovalAuthority,
    digest: Mutex<Option<Digest>>,
}
impl MigrationAuthority for RecordMigration<'_> {
    fn verify(&self, request: &MigrationRequest) -> Result<MigrationAttestation, String> {
        *self.digest.lock().unwrap() = Some(request.digest().clone());
        self.authority.verify(request)
    }
}
struct Present;
impl CapabilityProbe for Present {
    fn observe(&self, pin: &guard_contract::VerifierPin) -> VerifierObservation {
        VerifierObservation::Available(pin.clone())
    }
}
fn contract_with_protection(
    bytes: &[u8],
    revision: &str,
    protected: bool,
) -> (CompiledContract, Arc<Ports>, ReceiptApprovalAuthority) {
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
        value["approvalRef"] = json!(format!("approval-{level:?}"));
        value["sourceRefs"][0]["path"] =
            serde_json::to_value(GitPath::from_bytes(b"architecture.json").unwrap()).unwrap();
        value["sourceRefs"][0]["contentDigest"] = json!(digest(&native));
        if protected {
            value["obligations"][0]["targets"][0]["protectedDigest"] = json!(digest(bytes));
        }
        if revision == "expanded" {
            let mut target = value["obligations"][0]["targets"][0].clone();
            target["path"] =
                serde_json::to_value(GitPath::from_bytes(b"src/Other.java").unwrap()).unwrap();
            value["obligations"][0]["targets"]
                .as_array_mut()
                .unwrap()
                .push(target);
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
    let request = CompileRequest {
        layers,
        context,
        action: Action::Merge,
    };
    let ports = Arc::new(Ports::new());
    let verifier = Arc::new(ApprovalVerifier::new(
        ports.clone(),
        ports.clone(),
        ports.clone(),
        ports.clone(),
    ));
    let authority = ReceiptApprovalAuthority::for_compile(
        &request,
        verifier,
        id("controller-1"),
        id("coding-agent"),
        Some(id("MIG-1")),
    )
    .unwrap();
    for (level, (digest, reference)) in approved {
        ports.approve(
            &reference,
            &digest,
            authority.requested_scope(),
            ApprovalPurpose::Layer {
                level: level.into(),
            },
        );
    }
    let recorder = RecordTask {
        authority: &authority,
        digest: Mutex::new(None),
    };
    let blocked = compile(&request, &registry, &Present, Some(&recorder)).unwrap();
    assert!(blocked.compiled().is_none());
    let final_digest = recorder
        .digest
        .into_inner()
        .unwrap()
        .expect("real compiler called exact final authorization port");
    ports.approve(
        &id("AUTH-1"),
        &final_digest,
        authority.requested_scope(),
        ApprovalPurpose::Task,
    );
    let compiled = compile(&request, &registry, &Present, Some(&authority)).unwrap();
    (
        compiled
            .compiled()
            .unwrap_or_else(|| panic!("{:?}", compiled.diagnostics()))
            .clone(),
        ports,
        authority,
    )
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
fn real_contract_compile_requires_exact_independent_layer_and_final_authorizations() {
    let (contract, ports, authority) = contract_with_protection(b"class Example {}\n", "1", false);
    assert_eq!(contract.authorization_authority(), &id("external-issuer"));
    assert_eq!(contract.approval_attestations().len(), 3);
    *ports.revoked.lock().unwrap() = true;
    assert!(
        authority
            .authorize_task(contract.context(), contract.digest(), Action::Merge)
            .is_err()
    );
}
#[test]
fn requirement_scope_reuses_authorization_for_new_candidate_but_new_plan_is_distinct() {
    let temp = fixture();
    let root = temp.path();
    let (contract, ports, authority) = contract_with_protection(b"class Example {}\n", "1", false);
    let receipt_before = ports.receipts.lock().unwrap()[&id("AUTH-1")]
        .receipt
        .clone();
    authority
        .authorize_task(contract.context(), contract.digest(), Action::Merge)
        .unwrap();
    let task = binding(&contract);
    let first = captured(root, &contract);
    let first_plan = freeze(root, &contract, &task, &first, budget()).unwrap();
    fs::write(
        root.join("src/Example.java"),
        b"class Example { int changed; }\n",
    )
    .unwrap();
    authority
        .authorize_task(contract.context(), contract.digest(), Action::Merge)
        .unwrap();
    assert_eq!(
        ports.receipts.lock().unwrap()[&id("AUTH-1")].receipt,
        receipt_before
    );
    let next = captured(root, &contract);
    let next_plan = freeze(root, &contract, &task, &next, budget()).unwrap();
    assert!(next_plan.plan().is_some());
    assert_ne!(first_plan.digest(), next_plan.digest());
    assert!(
        !first_plan
            .plan()
            .unwrap()
            .equivalent_inputs(next_plan.plan().unwrap())
    );
    assert_eq!(
        first_plan.binding().authorization_refs,
        next_plan.binding().authorization_refs
    );
}
#[test]
fn signed_migration_request_moves_real_sqlite_head_and_wrong_old_revision_cannot_reuse_it() {
    let temp = fixture();
    let root = temp.path();
    let (contract, _, _) = contract_with_protection(b"class Example {}\n", "1", false);
    let task = binding(&contract);
    let first = captured(root, &contract);
    let external = tempfile::tempdir().unwrap();
    let store =
        EvidenceStore::create(&external.path().join("registry"), StoreLimits::default()).unwrap();
    let active = freeze(root, &contract, &task, &first, budget())
        .unwrap()
        .register(&store, 5000, 1000)
        .unwrap();
    let (next_contract, ports, authority) =
        contract_with_protection(b"class Example {}\n", "2", false);
    let next_task = binding(&next_contract);
    let next = captured(root, &next_contract);
    let record = RecordMigration {
        authority: &authority,
        digest: Mutex::new(None),
    };
    assert!(
        migrate(
            &active,
            &store,
            5000,
            1001,
            root,
            &next_contract,
            &next_task,
            &next,
            budget(),
            Some(&record)
        )
        .is_err()
    );
    let request_digest = record
        .digest
        .into_inner()
        .unwrap()
        .expect("actual migrate request reached read-only authority");
    ports.approve(
        &id("MIG-1"),
        next_contract.digest(),
        authority.requested_scope(),
        ApprovalPurpose::Migration { request_digest },
    );
    let migrated = migrate(
        &active,
        &store,
        5000,
        1002,
        root,
        &next_contract,
        &next_task,
        &next,
        budget(),
        Some(&authority),
    )
    .unwrap();
    migrated.plan().assert_current(&store, root, 1003).unwrap();
    assert!(
        migrate(
            &active,
            &store,
            5000,
            1003,
            root,
            &next_contract,
            &next_task,
            &next,
            budget(),
            Some(&authority)
        )
        .is_err()
    );
}

struct RevokeAfterVerify<'a> {
    inner: &'a ReceiptApprovalAuthority,
    ports: Arc<Ports>,
}
impl MigrationAuthority for RevokeAfterVerify<'_> {
    fn verify(&self, request: &MigrationRequest) -> Result<MigrationAttestation, String> {
        let result = self.inner.verify(request);
        if result.is_ok() {
            *self.ports.revoked.lock().unwrap() = true;
        }
        result
    }
}
#[test]
fn revocation_after_initial_migration_verification_prevents_cas() {
    let temp = fixture();
    let root = temp.path();
    let (contract, _, _) = contract_with_protection(b"class Example {}\n", "1", false);
    let task = binding(&contract);
    let first = captured(root, &contract);
    let external = tempfile::tempdir().unwrap();
    let store =
        EvidenceStore::create(&external.path().join("registry"), StoreLimits::default()).unwrap();
    let active = freeze(root, &contract, &task, &first, budget())
        .unwrap()
        .register(&store, 5000, 1000)
        .unwrap();
    let (next_contract, ports, authority) =
        contract_with_protection(b"class Example {}\n", "2", false);
    let next_task = binding(&next_contract);
    let next = captured(root, &next_contract);
    let record = RecordMigration {
        authority: &authority,
        digest: Mutex::new(None),
    };
    assert!(
        migrate(
            &active,
            &store,
            5000,
            1001,
            root,
            &next_contract,
            &next_task,
            &next,
            budget(),
            Some(&record)
        )
        .is_err()
    );
    ports.approve(
        &id("MIG-1"),
        next_contract.digest(),
        authority.requested_scope(),
        ApprovalPurpose::Migration {
            request_digest: record.digest.into_inner().unwrap().unwrap(),
        },
    );
    let revoking = RevokeAfterVerify {
        inner: &authority,
        ports,
    };
    assert!(
        migrate(
            &active,
            &store,
            5000,
            1002,
            root,
            &next_contract,
            &next_task,
            &next,
            budget(),
            Some(&revoking)
        )
        .is_err(),
        "revocation after initial signature check must prevent head publication"
    );
    active.assert_current(&store, root, 1003).unwrap();
}

#[test]
fn smaller_signed_scope_cannot_authorize_larger_actual_contract() {
    let (contract, ports, authority) =
        contract_with_protection(b"class Example {}\n", "expanded", false);
    assert_eq!(authority.requested_scope().targets.len(), 2);
    let mut smaller = authority.requested_scope().clone();
    smaller.targets.pop();
    ports.approve(
        &id("AUTH-1"),
        contract.digest(),
        &smaller,
        ApprovalPurpose::Task,
    );
    assert!(
        authority
            .authorize_task(contract.context(), contract.digest(), Action::Merge)
            .is_err()
    );
}
struct ChangeApprovalOnSecondCall<'a> {
    inner: &'a ReceiptApprovalAuthority,
    calls: Mutex<usize>,
}
impl MigrationAuthority for ChangeApprovalOnSecondCall<'_> {
    fn verify(&self, request: &MigrationRequest) -> Result<MigrationAttestation, String> {
        let mut approval = self.inner.verify(request)?;
        let mut calls = self.calls.lock().unwrap();
        *calls += 1;
        if *calls > 1 {
            approval.approval_ref = id("different-approval");
        }
        Ok(approval)
    }
}
#[test]
fn changed_approval_identity_at_migration_publish_boundary_prevents_cas() {
    let temp = fixture();
    let root = temp.path();
    let (contract, _, _) = contract_with_protection(b"class Example {}\n", "1", false);
    let task = binding(&contract);
    let first = captured(root, &contract);
    let external = tempfile::tempdir().unwrap();
    let store =
        EvidenceStore::create(&external.path().join("registry"), StoreLimits::default()).unwrap();
    let active = freeze(root, &contract, &task, &first, budget())
        .unwrap()
        .register(&store, 5000, 1000)
        .unwrap();
    let (next_contract, ports, authority) =
        contract_with_protection(b"class Example {}\n", "2", false);
    let next_task = binding(&next_contract);
    let next = captured(root, &next_contract);
    let record = RecordMigration {
        authority: &authority,
        digest: Mutex::new(None),
    };
    assert!(
        migrate(
            &active,
            &store,
            5000,
            1001,
            root,
            &next_contract,
            &next_task,
            &next,
            budget(),
            Some(&record)
        )
        .is_err()
    );
    ports.approve(
        &id("MIG-1"),
        next_contract.digest(),
        authority.requested_scope(),
        ApprovalPurpose::Migration {
            request_digest: record.digest.into_inner().unwrap().unwrap(),
        },
    );
    let changing = ChangeApprovalOnSecondCall {
        inner: &authority,
        calls: Mutex::new(0),
    };
    assert!(
        migrate(
            &active,
            &store,
            5000,
            1002,
            root,
            &next_contract,
            &next_task,
            &next,
            budget(),
            Some(&changing)
        )
        .unwrap_err()
        .contains("identity changed")
    );
    assert_eq!(*changing.calls.lock().unwrap(), 2);
    active.assert_current(&store, root, 1003).unwrap();
}

struct ReplaceSignedReceiptAfterVerify<'a> {
    inner: &'a ReceiptApprovalAuthority,
    ports: Arc<Ports>,
    calls: Mutex<usize>,
}
impl MigrationAuthority for ReplaceSignedReceiptAfterVerify<'_> {
    fn verify(&self, request: &MigrationRequest) -> Result<MigrationAttestation, String> {
        let approval = self.inner.verify(request)?;
        let mut calls = self.calls.lock().unwrap();
        *calls += 1;
        if *calls == 1 {
            let mut receipts = self.ports.receipts.lock().unwrap();
            let bundle = receipts.get_mut(&id("MIG-1")).unwrap();
            let ProtocolDocument::ApprovalReceipt(mut receipt) =
                guard_protocol::decode(&bundle.receipt)
                    .unwrap()
                    .document()
                    .clone()
            else {
                panic!("receipt")
            };
            receipt.principal = id("reviewer-2");
            let pair = Ed25519KeyPair::from_seed_unchecked(&[37; 32]).unwrap();
            receipt.proof.signature =
                id(&STANDARD.encode(pair.sign(&signing_bytes(&receipt).unwrap()).as_ref()));
            bundle.receipt =
                serde_json::to_vec(&ProtocolDocument::ApprovalReceipt(receipt)).unwrap();
        }
        Ok(approval)
    }
}
#[test]
fn replacing_valid_signed_receipt_under_same_id_cannot_cross_migration_cas() {
    let temp = fixture();
    let root = temp.path();
    let (contract, _, _) = contract_with_protection(b"class Example {}\n", "1", false);
    let task = binding(&contract);
    let first = captured(root, &contract);
    let external = tempfile::tempdir().unwrap();
    let store =
        EvidenceStore::create(&external.path().join("registry"), StoreLimits::default()).unwrap();
    let active = freeze(root, &contract, &task, &first, budget())
        .unwrap()
        .register(&store, 5000, 1000)
        .unwrap();
    let (next_contract, ports, authority) =
        contract_with_protection(b"class Example {}\n", "2", false);
    let next_task = binding(&next_contract);
    let next = captured(root, &next_contract);
    let record = RecordMigration {
        authority: &authority,
        digest: Mutex::new(None),
    };
    assert!(
        migrate(
            &active,
            &store,
            5000,
            1001,
            root,
            &next_contract,
            &next_task,
            &next,
            budget(),
            Some(&record)
        )
        .is_err()
    );
    ports.approve(
        &id("MIG-1"),
        next_contract.digest(),
        authority.requested_scope(),
        ApprovalPurpose::Migration {
            request_digest: record.digest.into_inner().unwrap().unwrap(),
        },
    );
    let changing = ReplaceSignedReceiptAfterVerify {
        inner: &authority,
        ports,
        calls: Mutex::new(0),
    };
    assert!(
        migrate(
            &active,
            &store,
            5000,
            1002,
            root,
            &next_contract,
            &next_task,
            &next,
            budget(),
            Some(&changing)
        )
        .is_err(),
        "another independently valid signed receipt under the same ID must not publish the head"
    );
    active.assert_current(&store, root, 1003).unwrap();
}
