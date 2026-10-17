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
use guard_run_ledger::RunState;
use guard_runner::{Cancellation, CommandSpec};
use guard_runtime::{DispatchOutcome, NativeVerifierCommand, Runtime};
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
    let exe = Path::new(env!("CARGO_BIN_EXE_guard-authority-source-fixture"))
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

use guard_authority::{
    AuthorityClock, ProducerRegistration, ProducerRegistry, ProducerSigner, ProducerSigningRequest,
    ProducerVerifier, RevocationSnapshot, RevocationSource,
};
use guard_protocol::UtcTimestamp;
use ring::signature::{Ed25519KeyPair, KeyPair};
use std::sync::Mutex;
fn timestamp(ms: u64) -> UtcTimestamp {
    UtcTimestamp::new(
        chrono::DateTime::<chrono::Utc>::from_timestamp_millis(ms as i64)
            .unwrap()
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
    )
    .unwrap()
}
struct SourcePorts {
    registration: Mutex<ProducerRegistration>,
    at: Mutex<UtcTimestamp>,
    revoked: Mutex<bool>,
    revision: Mutex<u64>,
    revocation_at: u64,
}
impl ProducerRegistry for SourcePorts {
    fn current(
        &self,
        _: &Identifier,
        _: &Identifier,
        _: &Identifier,
        _: &Identifier,
    ) -> Result<ProducerRegistration, String> {
        Ok(self.registration.lock().unwrap().clone())
    }
}
impl AuthorityClock for SourcePorts {
    fn now(&self) -> Result<UtcTimestamp, String> {
        Ok(self.at.lock().unwrap().clone())
    }
}
impl RevocationSource for SourcePorts {
    fn current(
        &self,
        issuer: &Identifier,
        key: &Identifier,
        receipt: &Identifier,
        reference: &Identifier,
    ) -> Result<RevocationSnapshot, String> {
        let revision = *self.revision.lock().unwrap();
        Ok(RevocationSnapshot {
            authority: id("source-revocation"),
            issuer: issuer.clone(),
            key_id: key.clone(),
            receipt_id: receipt.clone(),
            revocation_ref: reference.clone(),
            revision,
            state_digest: digest(&revision.to_le_bytes()),
            valid_from: timestamp(self.revocation_at - 600000),
            expires_at: timestamp(self.revocation_at + 600000),
            revoked: *self.revoked.lock().unwrap(),
        })
    }
}
struct IndependentSigner;
impl ProducerSigner for IndependentSigner {
    fn attest(&self, request: &ProducerSigningRequest) -> Result<Vec<u8>, String> {
        // 独立从wire剔除signature并拼固定域，避免签名/验签共用错误公式。
        let blank = request.complete(&[0; 64])?;
        let mut json: Value = serde_json::from_slice(&blank).unwrap();
        json["proof"].as_object_mut().unwrap().remove("signature");
        let mut independent = b"guard.producer-attestation/v1\n".to_vec();
        independent
            .extend(guard_protocol::canonicalize(&serde_json::to_vec(&json).unwrap()).unwrap());
        assert_eq!(independent, request.signing_bytes()?);
        let pair = Ed25519KeyPair::from_seed_unchecked(&[91; 32]).unwrap();
        request.complete(pair.sign(&independent).as_ref())
    }
}
#[tokio::test]
async fn real_failed_runtime_source_has_independent_signature_and_never_quality_pass() {
    let temp = fixture();
    let root = temp.path();
    let external = tempfile::tempdir().unwrap();
    let cmd = command(root, "fail", &external.path().join("source-marker"));
    let contract = contract(&cmd);
    let bind = binding(&contract);
    let subject = captured(root, &contract);
    let store = Arc::new(
        EvidenceStore::create(&external.path().join("store"), StoreLimits::default()).unwrap(),
    );
    let end = now() + 600000;
    let plan = freeze(root, &contract, &bind, &subject, budget())
        .unwrap()
        .register(&store, end, now())
        .unwrap();
    let runtime = Runtime::new(&plan, &contract, store.clone(), root, end).unwrap();
    let obligation = plan.plan().verification().obligations[0]
        .obligation_id
        .clone();
    let (outcome, completion) = runtime
        .execute_with_receipt(
            &obligation,
            "source-run",
            native(cmd.clone()),
            Cancellation::new(),
        )
        .await
        .unwrap()
        .into_parts();
    let completion = completion.expect("actual failed process receipt");
    let DispatchOutcome::Finished(observed) = outcome else {
        panic!("not observed")
    };
    assert_eq!(observed.exit_code, Some(7));
    assert_eq!(observed.state, RunState::Fail);
    assert_eq!(observed.trust, "local_unverified");
    let pin = contract
        .verifiers()
        .iter()
        .find(|p| p.verifier_id == id("java-package-check"))
        .unwrap();
    let pair = Ed25519KeyPair::from_seed_unchecked(&[91; 32]).unwrap();
    // 受保护配置来自控制端已知fixture/编译pin/命令，绝不从候选结果导出注册表。
    let registration = ProducerRegistration {
        issuer: id("independent-source"),
        key_id: id("source-key"),
        public_key: pair.public_key().as_ref().try_into().unwrap(),
        revocation_authority: id("source-revocation"),
        revocation_ref: id("source-list"),
        audience: id("gate-source"),
        repository_id: contract.context().repository_id.clone(),
        task_id: bind.task_id.clone(),
        contract_digest: contract.digest().clone(),
        policy_digest: plan.plan().verification().policy_digest.clone(),
        scope_digest: guard_protocol::digest_json(&serde_json::to_vec(&bind.scope).unwrap())
            .unwrap(),
        producer: guard_protocol::ProducerIdentity {
            schema_version: SchemaVersion::new("1.0.0").unwrap(),
            producer_id: pin.verifier_id.clone(),
            version: pin.version.clone(),
            artifact_digest: pin.artifact_digest.clone(),
            config_digest: pin.config_digest.clone(),
            adapter_version: pin.adapter_version.clone(),
            execution_environment_digest: plan.capture_binding().environment_digest.clone(),
            extensions: Extensions::default(),
        },
        domain: DomainOwner::Archguard,
        verifier_pin_digest: guard_protocol::digest_json(&serde_json::to_vec(pin).unwrap())
            .unwrap(),
        profile: "guard.native-command/v1".into(),
        approved_invocation: guard_runtime::SourceInvocation {
            executable: cmd.executable.to_str().unwrap().into(),
            artifact_digest: cmd.executable_digest.clone(),
            argv: cmd.args.clone(),
            working_directory: cmd.directory.to_str().unwrap().into(),
            environment: cmd.environment.clone(),
        },
        allowed_subject_kinds: vec![guard_protocol::SubjectKind::Worktree],
        max_validity_ms: 60000,
    };
    let at = now();
    let ports = Arc::new(SourcePorts {
        registration: Mutex::new(registration),
        at: Mutex::new(timestamp(at)),
        revoked: Mutex::new(false),
        revision: Mutex::new(1),
        revocation_at: at,
    });
    let verifier = ProducerVerifier::new(ports.clone(), ports.clone(), ports.clone());
    let request = verifier
        .prepare(
            &completion,
            id("SOURCE-1"),
            id("gate-source"),
            timestamp(at + 30000),
        )
        .unwrap();
    let raw = IndependentSigner.attest(&request).unwrap();
    let verified = verifier
        .verify_live(&raw, &completion, &id("gate-source"))
        .expect("genuine independently signed same-runtime bytes must verify");
    assert_eq!(verified.attestation().provenance.exit_code, Some(7));
    assert_eq!(verified.attestation().provenance.state, RunState::Fail);
    let context = guard_authority::ProducerRestoreContext {
        plan: &plan,
        contract: &contract,
        store: &store,
        project: root,
    };
    let restarted = ProducerVerifier::new(ports.clone(), ports.clone(), ports.clone());
    restarted
        .verify_restored(&raw, &context, &id("gate-source"))
        .expect("restart requires valid independent signature and current real originals");
    // 所有负例仍使用实际Runtime原件；只变一个签名声明或受保护注册项。
    for case in 0..11 {
        let mut value: Value = serde_json::from_slice(&raw).unwrap();
        match case {
            0 => value["audience"] = json!("candidate-audience"),
            1 => value["issuer"] = json!("candidate-issuer"),
            2 => value["proof"]["keyId"] = json!("candidate-key"),
            3 => value["schemaVersion"] = json!("guard.producer-attestation/v99"),
            4 => value["provenance"]["profile"] = json!("unknown-profile"),
            5 => value["provenance"]["producer"]["adapterVersion"] = json!("99.0.0"),
            6 => value["provenance"]["domain"] = json!("codeguard"),
            7 => value["provenance"]["subjectKind"] = json!("index"),
            8 => value["provenance"]["approvedInvocation"]["argv"] = json!(["forged-argv"]),
            9 => {
                value["provenance"]["effectiveInvocation"]["environment"] =
                    json!({"INJECTED":"true"})
            }
            _ => value["assessment"] = json!({"verdict":"SATISFIED"}),
        }
        let changed = independently_resign(value);
        assert!(
            verifier
                .verify_live(&changed, &completion, &id("gate-source"))
                .is_err(),
            "signed negative {case}"
        );
    }
    let original_registration = ports.registration.lock().unwrap().clone();
    ports
        .registration
        .lock()
        .unwrap()
        .approved_invocation
        .argv
        .push("unapproved".into());
    assert!(
        verifier
            .verify_live(&raw, &completion, &id("gate-source"))
            .is_err()
    );
    *ports.registration.lock().unwrap() = original_registration.clone();
    ports
        .registration
        .lock()
        .unwrap()
        .approved_invocation
        .environment
        .insert("INJECTED".into(), "value".into());
    assert!(
        verifier
            .verify_live(&raw, &completion, &id("gate-source"))
            .is_err()
    );
    *ports.registration.lock().unwrap() = original_registration.clone();
    ports.registration.lock().unwrap().public_key = [0; 32];
    assert!(
        verifier
            .verify_live(&raw, &completion, &id("gate-source"))
            .is_err()
    );
    *ports.registration.lock().unwrap() = original_registration;
    let mut tampered: Value = serde_json::from_slice(&raw).unwrap();
    tampered["expiresAt"] = json!(timestamp(at + 29000));
    let changed = independently_resign(tampered);
    assert!(
        verifier
            .verify_live(&changed, &completion, &id("gate-source"))
            .err()
            .unwrap()
            .contains("changed")
    );
    assert!(
        verifier
            .verify_live(&raw, &completion, &id("gate-source"))
            .is_err(),
        "restoring bytes cannot erase observed same-ID conflict"
    );
    // 新运行/手造新头使之前已签原件过期，签名正确也不能跨头使用。
    let ledger = guard_run_ledger::RunLedger::new(&store);
    let ticket = ledger
        .start(
            &completion.provenance().scope,
            &completion.provenance().identity,
            "source-forged-new-head",
            end,
            now(),
        )
        .unwrap()
        .into_ticket();
    let old_metadata = store
        .read_private(&completion.provenance().metadata.operation, now())
        .unwrap();
    assert_eq!(
        ledger
            .finalize(&ticket, RunState::Pass, &old_metadata, now())
            .unwrap(),
        guard_run_ledger::FinalizeOutcome::Applied
    );
    assert!(
        restarted
            .verify_restored(&raw, &context, &id("gate-source"))
            .err()
            .unwrap()
            .contains("source run head/metadata no longer current"),
        "valid signature cannot restore stale lineage"
    );
    let (second, completion2) = runtime
        .execute_with_receipt(
            &obligation,
            "source-run-2",
            native(cmd),
            Cancellation::new(),
        )
        .await
        .unwrap()
        .into_parts();
    let completion2 = completion2.unwrap();
    let DispatchOutcome::Finished(second) = second else {
        panic!("not run")
    };
    let request2 = verifier
        .prepare(
            &completion2,
            id("SOURCE-2"),
            id("gate-source"),
            timestamp(at + 30000),
        )
        .unwrap();
    let raw2 = IndependentSigner.attest(&request2).unwrap();
    verifier
        .verify_live(&raw2, &completion2, &id("gate-source"))
        .unwrap();
    // 私有stdout损坏被真实store隔离，恢复字节也不恢复来源资格。
    let path = store.artifact_path(&second.stdout.operation).unwrap();
    let original = fs::read(&path).unwrap();
    fs::write(&path, b"replacement").unwrap();
    assert!(
        restarted
            .verify_restored(&raw2, &context, &id("gate-source"))
            .is_err()
    );
    fs::write(&path, original).unwrap();
    assert!(
        restarted
            .verify_restored(&raw2, &context, &id("gate-source"))
            .is_err()
    );
    *ports.revoked.lock().unwrap() = true;
    *ports.revision.lock().unwrap() = 2;
    assert!(
        verifier
            .verify_live(&raw2, &completion2, &id("gate-source"))
            .err()
            .unwrap()
            .contains("attestation revoked")
    );
    *ports.revoked.lock().unwrap() = false;
    *ports.revision.lock().unwrap() = 3;
    assert!(
        verifier
            .verify_live(&raw2, &completion2, &id("gate-source"))
            .err()
            .unwrap()
            .contains("attestation revoked"),
        "higher active revision cannot revive observed revocation"
    );
    *ports.at.lock().unwrap() = timestamp(at + 60000);
    assert!(
        ProducerVerifier::new(ports.clone(), ports.clone(), ports.clone())
            .verify_restored(&raw2, &context, &id("gate-source"))
            .err()
            .unwrap()
            .contains("attestation expired/future/invalid lifetime")
    );
}

fn independently_resign(mut value: Value) -> Vec<u8> {
    use base64::{Engine, engine::general_purpose::STANDARD};
    value["proof"].as_object_mut().unwrap().remove("signature");
    value["proof"]["provenanceDigest"] = json!(
        guard_protocol::digest_json(&serde_json::to_vec(&value["provenance"]).unwrap()).unwrap()
    );
    let mut payload = b"guard.producer-attestation/v1\n".to_vec();
    payload.extend(guard_protocol::canonicalize(&serde_json::to_vec(&value).unwrap()).unwrap());
    let pair = Ed25519KeyPair::from_seed_unchecked(&[91; 32]).unwrap();
    value["proof"]["signature"] = json!(STANDARD.encode(pair.sign(&payload).as_ref()));
    serde_json::to_vec(&value).unwrap()
}
