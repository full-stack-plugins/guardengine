#![allow(dead_code)]
#[path = "../src/bootstrap.rs"]
mod bootstrap;
#[path = "../src/cli_failure.rs"]
mod cli_failure;
#[path = "../src/configured_verifier.rs"]
mod configured_verifier;
#[path = "../src/file_ports.rs"]
mod file_ports;
#[path = "../src/host_config.rs"]
mod host_config;
#[path = "../src/prepared.rs"]
mod prepared;
#[path = "../src/probe_results.rs"]
mod probe_results;
#[path = "../src/safe_files.rs"]
mod safe_files;
use base64::{Engine, engine::general_purpose::STANDARD};
use guard_authority::{
    ApprovalMaterial, ApprovalPurpose, ApprovalVerifier, ApprovedScope, ReceiptApprovalAuthority,
    signing_bytes,
};
use guard_contract::{
    ApprovalAttestation, ApprovalAuthority, CompileContext, LayerLevel, TaskAuthorization, compile,
    snapshot_digest,
};
use guard_protocol::{Action, ApprovalIntent, Digest, Identifier, ProtocolDocument};
use ring::signature::{Ed25519KeyPair, KeyPair};
use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::{Arc, Mutex},
};
fn id(s: &str) -> Identifier {
    Identifier::new(s).unwrap()
}
fn hash(b: &[u8]) -> Digest {
    Digest::new(format!("sha256:{:x}", Sha256::digest(b))).unwrap()
}
fn write(path: &Path, value: &Value) {
    fs::write(path, serde_json::to_vec_pretty(value).unwrap()).unwrap();
}
fn cli(args: &[&str]) -> (i32, Value) {
    let out = Command::new(env!("CARGO_BIN_EXE_guardcore"))
        .args(args)
        .output()
        .unwrap();
    parse_output(out)
}
fn parse_output(out: Output) -> (i32, Value) {
    let value = serde_json::from_slice(&out.stdout).unwrap_or_else(|_| {
        panic!(
            "stdout={} stderr={}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        )
    });
    (out.status.code().unwrap(), value)
}
fn expect(code: i32, args: &[&str]) -> Value {
    let (actual, value) = cli(args);
    assert_eq!(actual, code, "{args:?}: {value}");
    value
}
struct OwnedHostEntry {
    path: PathBuf,
    prefix: String,
}
impl Drop for OwnedHostEntry {
    fn drop(&mut self) {
        if let Ok(bytes) = fs::read(&self.path) {
            if let Ok(value) = serde_json::from_slice::<Value>(&bytes) {
                if value["layers"][0]["document"]
                    .as_str()
                    .is_some_and(|v| v.starts_with(&format!("{}/", self.prefix)))
                {
                    let _ = fs::remove_file(&self.path);
                }
            }
        }
    }
}
struct Recorder<'a> {
    inner: &'a ReceiptApprovalAuthority,
    digest: Mutex<Option<Digest>>,
}
impl ApprovalAuthority for Recorder<'_> {
    fn verify_layer(
        &self,
        c: &CompileContext,
        l: LayerLevel,
        r: &Identifier,
        d: &Digest,
        a: Action,
    ) -> Result<ApprovalAttestation, String> {
        self.inner.verify_layer(c, l, r, d, a)
    }
    fn authorize_task(
        &self,
        c: &CompileContext,
        d: &Digest,
        a: Action,
    ) -> Result<TaskAuthorization, String> {
        *self.digest.lock().unwrap() = Some(d.clone());
        self.inner.authorize_task(c, d, a)
    }
}
fn sign(
    root: &Path,
    prefix: &str,
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
    let ProtocolDocument::ApprovalReceipt(mut receipt) = guard_protocol::decode(include_bytes!(
        "../../guard-protocol/fixtures/valid/approval-receipt.json"
    ))
    .unwrap()
    .document()
    .clone() else {
        panic!()
    };
    receipt.receipt_id = reference.clone();
    receipt.intent = ApprovalIntent::RequirementScope;
    receipt.scope_digest = scope.digest().unwrap();
    receipt.contract_digest = digest.clone();
    receipt.subject_binding.kind = receipt.intent;
    receipt.subject_binding.scope_digest = receipt.scope_digest.clone();
    receipt.subject_binding.subject_digest = None;
    receipt.issued_at = guard_protocol::UtcTimestamp::new(
        (chrono::Utc::now() - chrono::Duration::minutes(1))
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
    )
    .unwrap();
    receipt.expires_at = guard_protocol::UtcTimestamp::new(
        (chrono::Utc::now() + chrono::Duration::hours(1))
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
    )
    .unwrap();
    receipt.proof.provenance_digest =
        guard_protocol::digest_json(&serde_json::to_vec(&material).unwrap()).unwrap();
    let pair = Ed25519KeyPair::from_seed_unchecked(&[97; 32]).unwrap();
    receipt.proof.signature =
        id(&STANDARD.encode(pair.sign(&signing_bytes(&receipt).unwrap()).as_ref()));
    write(
        &root.join(format!("{prefix}/{}.receipt.json", reference.as_str())),
        &serde_json::to_value(ProtocolDocument::ApprovalReceipt(receipt.clone())).unwrap(),
    );
    write(
        &root.join(format!("{prefix}/{}.material.json", reference.as_str())),
        &serde_json::to_value(material).unwrap(),
    );
    write(
        &root.join(format!("{prefix}/{}.revocation.json", reference.as_str())),
        &json!({"authority":"protected-revocation","issuer":receipt.issuer,"keyId":receipt.proof.key_id,"receiptId":reference,"revocationRef":receipt.revocation_ref,"revision":1,"stateDigest":hash(b"current-independent-list"),"validFrom":receipt.issued_at,"expiresAt":receipt.expires_at,"revoked":false}),
    );
}

#[tokio::test]
async fn real_cli_uses_independent_receipts_private_candidate_and_pinned_opa_without_legacy_fallback()
 {
    let root = PathBuf::from(env!("GUARDCORE_TEST_HOST_ROOT"));
    assert!(root.is_absolute());
    fs::create_dir_all(&root).unwrap();
    let sentinel = root.join(".guard-cli-owned-test-root");
    if sentinel.exists() {
        assert_eq!(fs::read(&sentinel).unwrap(), b"guard-cli-test-v1");
    } else {
        assert!(
            fs::read_dir(&root).unwrap().next().is_none(),
            "never overwrite user host files"
        );
        fs::write(&sentinel, b"guard-cli-test-v1").unwrap();
    }
    assert!(
        !root.join("host.json").exists(),
        "prior host file must not be overwritten"
    );
    let trust = tempfile::Builder::new()
        .prefix("authority-")
        .tempdir_in(&root)
        .unwrap();
    let prefix = trust.path().file_name().unwrap().to_str().unwrap();
    let temporary = tempfile::tempdir().unwrap();
    let project = temporary.path().join("project");
    let scratch = temporary.path().join("scratch");
    fs::create_dir_all(project.join("src")).unwrap();
    fs::create_dir(&scratch).unwrap();
    fs::write(project.join("src/Example.java"), b"COMMITTED").unwrap();
    let git = std::env::var("GUARDCORE_TEST_GIT_EXE").expect("explicit real Git required");
    for args in [
        vec!["init", "-q"],
        vec!["config", "user.email", "fixture@example.invalid"],
        vec!["config", "user.name", "Fixture"],
        vec!["config", "core.autocrlf", "false"],
        vec!["add", "."],
        vec!["commit", "-qm", "fixture"],
    ] {
        assert!(
            Command::new(&git)
                .current_dir(&project)
                .args(args)
                .status()
                .unwrap()
                .success()
        );
    }
    let base = String::from_utf8(
        Command::new(&git)
            .current_dir(&project)
            .args(["rev-parse", "HEAD"])
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap()
    .trim()
    .to_owned();
    let native = PathBuf::from(env!("CARGO_BIN_EXE_guardcore-fixture"));
    let opa = PathBuf::from(
        std::env::var("GUARD_POLICY_OPA_EXECUTABLE").expect("official OPA path required"),
    );
    let opa_hash = hash(&fs::read(&opa).unwrap());
    assert_eq!(
        opa_hash.as_str(),
        std::env::var("GUARD_POLICY_OPA_SHA256").unwrap()
    );
    let version = Command::new(&opa).arg("version").output().unwrap();
    assert!(version.status.success());
    let marker = temporary.path().join("native-executions.txt");
    let git_descriptor =
        json!({"executable":git,"artifactDigest":hash(&fs::read(&git).unwrap()),"environment":{}});
    let git_marker=guard_protocol::digest_json(&serde_json::to_vec(&json!({"schemaVersion":"guard.cli-git-provider/v1","platform":{"os":std::env::consts::OS,"arch":std::env::consts::ARCH},"provider":git_descriptor})).unwrap()).unwrap();
    let baseline = hash(b"external approved fixture environment baseline");
    let environment = std::collections::BTreeMap::from([
        (
            "GIT_CEILING_DIRECTORIES".to_string(),
            "{{subject_parent}}".to_string(),
        ),
        (
            "GUARDCORE_GIT_PROVIDER_DIGEST".into(),
            git_marker.as_str().into(),
        ),
        (
            "GUARDCORE_ENVIRONMENT_BASELINE_DIGEST".into(),
            baseline.as_str().into(),
        ),
    ]);
    let native_command = guard_runner::CommandSpec {
        executable: native.clone(),
        executable_digest: hash(&fs::read(&native).unwrap()),
        args: vec!["check".into(), marker.to_str().unwrap().into()],
        directory: project.clone(),
        environment: environment.clone(),
    };
    let budget = guard_protocol::ExecutionBudget {
        timeout_ms: 30000,
        max_output_bytes: 65536,
        max_concurrency: 1,
    };
    fs::create_dir(trust.path().join("policy")).unwrap();
    let rego=b"package fixture\nimport rego.v1\ndecision := {\"schemaVersion\":1,\"decision\":input.facts.verdict,\"reasons\":[],\"requiredActions\":[],\"missingObligations\":[],\"activeViolations\":[]} if { input.facts.defined == true }\n";
    let caps = include_bytes!("fixtures/capabilities.json");
    fs::write(trust.path().join("policy/entry.rego"), rego).unwrap();
    fs::write(trust.path().join("policy/capabilities.json"), caps).unwrap();
    let manifest = json!({"schemaVersion":1,"packageId":"flowguard.cli-fixture","version":"1.0.0","owner":"flowguard","entrypoint":"data.fixture.decision","files":[{"path":"entry.rego","digest":hash(rego)}],"capabilities":{"path":"capabilities.json","digest":hash(caps)}});
    write(&trust.path().join("policy/manifest.json"), &manifest);
    let package = guard_policy::PolicyPackage::load(
        &trust.path().join("policy"),
        &serde_json::to_vec(&manifest).unwrap(),
    )
    .unwrap();
    let opa_config = guard_policy::OpaConfig {
        executable: opa.clone(),
        executable_digest: opa_hash.clone(),
        version: id("1.21.2"),
        environment: Default::default(),
        budget: budget.clone(),
    };
    let pair = Ed25519KeyPair::from_seed_unchecked(&[97; 32]).unwrap();
    let source = b"approved architecture fixture";
    fs::write(trust.path().join("source.txt"), source).unwrap();
    let mut layers = Vec::new();
    let mut layer_files = Vec::new();
    for (level, name) in [
        (LayerLevel::Organization, "organization"),
        (LayerLevel::Project, "project"),
        (LayerLevel::Task, "task"),
    ] {
        let mut value: Value = serde_json::from_slice(include_bytes!(
            "../../guard-protocol/fixtures/valid/contract-snapshot.json"
        ))
        .unwrap();
        value["contractId"] = json!(name);
        value["approvalRef"] = json!(format!("approval-{name}"));
        value["sourceRefs"][0]["contentDigest"] = json!(hash(source));
        let ProtocolDocument::ContractSnapshot(mut snapshot) =
            guard_protocol::decode(&serde_json::to_vec(&value).unwrap())
                .unwrap()
                .document()
                .clone()
        else {
            panic!()
        };
        snapshot.digest = snapshot_digest(&snapshot).unwrap();
        let document =
            serde_json::to_vec(&ProtocolDocument::ContractSnapshot(snapshot.clone())).unwrap();
        fs::write(trust.path().join(format!("{name}.json")), &document).unwrap();
        layers.push(guard_contract::ContractLayer {
            level,
            document,
            sources: vec![guard_contract::SourceMaterial {
                source: snapshot.source_refs[0].clone(),
                bytes: source.to_vec(),
            }],
        });
        layer_files.push(json!({"level":name,"document":format!("{prefix}/{name}.json"),"sources":{hash(source).as_str():format!("{prefix}/source.txt")}}));
    }
    let mut host = json!({"schemaVersion":"guard.cli-host/v1","project":project,"repositoryId":"repo-example","taskId":"REQ-1","actor":"coding-agent","audience":"controller-1","worktreeId":"worktree-1","action":"MERGE","authorizationRefs":["AUTH-1"],"taskBinding":format!("{prefix}/binding.json"),"layers":layer_files,"policy":format!("{prefix}/policy"),"stateRoot":temporary.path().join("state"),"scratchRoot":scratch,"retentionMs":86400000,"budget":budget,"environmentDigest":baseline,"anchors":[{"keyId":"external-key-1","issuer":"external-issuer","publicKey":STANDARD.encode(pair.public_key().as_ref()),"revocationAuthority":"protected-revocation","identities":[{"principal":"reviewer-1","role":"architect","purposes":["organization_layer","project_layer","task_layer","task"],"intents":["REQUIREMENT_SCOPE"]}],"audiences":["controller-1"],"intents":["REQUIREMENT_SCOPE"]}],"receipts":{},"revocations":{},"verifiers":[],"git":git_descriptor});
    for reference in [
        "approval-organization",
        "approval-project",
        "approval-task",
        "AUTH-1",
    ] {
        host["receipts"][reference] = json!({"receipt":format!("{prefix}/{reference}.receipt.json"),"material":format!("{prefix}/{reference}.material.json")});
        host["revocations"][reference] = json!(format!("{prefix}/{reference}.revocation.json"));
    }
    host["verifiers"] = json!([
    {"verifierId":"java-package-check","version":"1.0.0","artifactDigest":native_command.executable_digest,"configDigest":guard_runtime::NativeVerifierCommandV2::configuration_digest(&native_command).unwrap(),"adapterVersion":"1.0.0","kind":"domain","owner":"archguard","evidenceTypes":["source-report"],"parameterSchema":{"type":"object","properties":{"maxCycles":{"type":"integer"},"thresholdDecimal":{"type":"string"}},"required":["maxCycles","thresholdDecimal"],"additionalProperties":false},"executable":native,"argv":native_command.args,"environment":environment,"probeArgv":["--version"],"probeEnvironment":{},"probeStdout":"guard-cli-fixture/1\n","probeStderr":""},
    {"verifierId":"opa","version":"1.21.2","artifactDigest":opa_hash,"configDigest":opa_config.configuration_digest(&package).unwrap(),"adapterVersion":"guard-opa/v1","kind":"policy_engine","owner":null,"evidenceTypes":["policy-decision"],"parameterSchema":{"type":"object"},"executable":opa,"argv":[],"environment":{},"probeArgv":["version"],"probeEnvironment":{},"probeStdout":String::from_utf8(version.stdout).unwrap(),"probeStderr":String::from_utf8(version.stderr).unwrap()}
    ]);
    write(&root.join("host.json"), &host);
    let _owned = OwnedHostEntry {
        path: root.join("host.json"),
        prefix: prefix.to_owned(),
    };
    let config = host_config::HostConfig::load().unwrap();
    let request = guard_contract::CompileRequest {
        layers,
        context: CompileContext {
            repository_id: id("repo-example"),
            task_id: id("REQ-1"),
            authorization_refs: vec![id("AUTH-1")],
        },
        action: Action::Merge,
    };
    for field in ["actor", "taskId"] {
        let mut changed = host.clone();
        changed[field] = json!("different-host-context");
        write(&root.join("host.json"), &changed);
        let error = guard_authority::ReceiptSource::get(
            &file_ports::FilePorts(config.clone()),
            &id("approval-organization"),
        )
        .unwrap_err();
        assert!(error.contains("host profile changed"), "{error}");
        write(&root.join("host.json"), &host);
    }
    let ports = Arc::new(file_ports::FilePorts(config.clone()));
    let verifier = Arc::new(ApprovalVerifier::new(
        ports.clone(),
        ports.clone(),
        ports.clone(),
        ports,
    ));
    let authority = ReceiptApprovalAuthority::for_compile(
        &request,
        verifier,
        id("controller-1"),
        id("coding-agent"),
        None,
    )
    .unwrap();
    for layer in &request.layers {
        let ProtocolDocument::ContractSnapshot(snapshot) = guard_protocol::decode(&layer.document)
            .unwrap()
            .document()
            .clone()
        else {
            panic!()
        };
        sign(
            &root,
            prefix,
            &snapshot.approval_ref,
            &snapshot.digest,
            authority.requested_scope(),
            ApprovalPurpose::Layer {
                level: layer.level.into(),
            },
        );
    }
    let (registry, probes, _) = prepared::probe(&config).await.unwrap();
    let recorder = Recorder {
        inner: &authority,
        digest: Mutex::new(None),
    };
    assert!(
        compile(&request, &registry, &probes, Some(&recorder))
            .unwrap()
            .compiled()
            .is_none()
    );
    sign(
        &root,
        prefix,
        &id("AUTH-1"),
        &recorder.digest.into_inner().unwrap().unwrap(),
        authority.requested_scope(),
        ApprovalPurpose::Task,
    );
    let ready = compile(&request, &registry, &probes, Some(&authority)).unwrap();
    let contract = ready
        .compiled()
        .unwrap_or_else(|| panic!("{:?}", ready.diagnostics()));
    let mut binding: Value = serde_json::from_slice(include_bytes!(
        "../../guard-protocol/fixtures/valid/task-binding.json"
    ))
    .unwrap();
    binding["contractDigest"] = json!(contract.digest());
    binding["authorizationRefs"] = json!(["AUTH-1"]);
    binding["requirementRefs"] = json!(contract.source_refs());
    write(&trust.path().join("binding.json"), &binding);
    let select = temporary.path().join("selection.json");
    write(&select, &json!({"kind":"commit","commit":base,"base":base}));
    let selection = select.to_str().unwrap();
    let doctor = expect(0, &["doctor"]);
    assert_eq!(
        doctor["result"]["discovery"]["skills"]["status"],
        "not_inspected"
    );
    assert_eq!(
        doctor["result"]["discovery"]["languageBuild"]["semanticCompatibility"],
        "not_inspected"
    );
    assert_eq!(
        doctor["result"]["authority"],
        "configured_not_yet_validated"
    );
    expect(
        0,
        &[
            "contract",
            "validate",
            "--file",
            trust.path().join("task.json").to_str().unwrap(),
        ],
    );
    expect(0, &["contract", "compile"]);
    expect(0, &["plan", "preview", "--selection", selection]);
    expect(
        0,
        &[
            "plan",
            "register",
            "--selection",
            selection,
            "--operation",
            "plan-commit",
        ],
    );
    expect(
        0,
        &[
            "plan",
            "restore",
            "--selection",
            selection,
            "--operation",
            "plan-commit",
        ],
    );
    fs::write(project.join("src/Example.java"), b"DIRTY-WORKTREE").unwrap();
    let result = expect(
        0,
        &[
            "run",
            "--selection",
            selection,
            "--plan-operation",
            "plan-commit",
            "--operation",
            "native-1",
            "--obligation",
            "OB-1",
        ],
    );
    assert_eq!(result["quality"], "not_evaluated");
    assert_eq!(result["result"]["observation"]["exitCode"], 0);
    let op = result["result"]["observation"]["stdout"]["operation"]
        .as_str()
        .unwrap();
    let raw = expect(0, &["evidence", "show", "--operation", op, "--raw"]);
    assert_eq!(
        STANDARD
            .decode(raw["result"]["privateRaw"]["bytes"].as_str().unwrap())
            .unwrap(),
        b"OBSERVED=COMMITTED\n"
    );
    assert!(!project.join("generated-by-fixture.txt").exists());
    let summary = expect(0, &["evidence", "show", "--operation", op]);
    assert!(summary["result"].get("privateRaw").is_none());
    expect(0, &["evidence", "verify", "--operation", op]);
    let before = fs::read(&marker).unwrap();
    expect(
        3,
        &[
            "run",
            "--selection",
            selection,
            "--plan-operation",
            "plan-commit",
            "--operation",
            "native-1",
            "--obligation",
            "OB-1",
        ],
    );
    assert_eq!(before, fs::read(&marker).unwrap());
    let fake = temporary.path().join("fake-git");
    fs::create_dir(&fake).unwrap();
    fs::copy(
        &native,
        fake.join(if cfg!(windows) { "git.exe" } else { "git" }),
    )
    .unwrap();
    let evil_marker = temporary.path().join("evil-git-executed");
    let out = Command::new(env!("CARGO_BIN_EXE_guardcore"))
        .args(["plan", "preview", "--selection", selection])
        .current_dir(&fake)
        .env(
            "PATH",
            std::env::join_paths(std::iter::once(fake.clone()).chain(std::env::split_paths(
                &std::env::var_os("PATH").unwrap_or_default(),
            )))
            .unwrap(),
        )
        .env("EVIL_GIT_MARKER", &evil_marker)
        .output()
        .unwrap();
    let (code, value) = parse_output(out);
    assert_eq!(code, 0, "{value}");
    assert!(
        !evil_marker.exists(),
        "fixed Git must ignore parent PATH/cwd fake executable"
    );
    expect(0, &["policy", "validate"]);
    let facts = temporary.path().join("facts.json");
    write(&facts, &json!({"defined":true,"verdict":"ALLOW"}));
    let evaluated = expect(
        0,
        &[
            "policy",
            "evaluate",
            "--selection",
            selection,
            "--plan-operation",
            "plan-commit",
            "--operation",
            "opa-1",
            "--facts",
            facts.to_str().unwrap(),
        ],
    );
    assert_eq!(
        evaluated["result"]["receipt"]["evaluation"]["decision"]["decision"],
        "ALLOW"
    );
    write(&facts, &json!({"defined":true,"verdict":"DENY"}));
    expect(
        1,
        &[
            "policy",
            "evaluate",
            "--selection",
            selection,
            "--plan-operation",
            "plan-commit",
            "--operation",
            "opa-deny",
            "--facts",
            facts.to_str().unwrap(),
        ],
    );
    write(&facts, &json!({"defined":false,"verdict":"ALLOW"}));
    expect(
        3,
        &[
            "policy",
            "evaluate",
            "--selection",
            selection,
            "--plan-operation",
            "plan-commit",
            "--operation",
            "opa-undefined",
            "--facts",
            facts.to_str().unwrap(),
        ],
    );
    // 实际SDK控制端持有opaque完成对象，CLI只接其独立签名原件，不跨进程恢复token。
    bootstrap::configure(&host_config::HostConfig::load().unwrap()).unwrap();
    let sdk = prepared::Prepared::compile(host_config::HostConfig::load().unwrap())
        .await
        .unwrap();
    let store = sdk.store(false).unwrap();
    let registered = sdk.restore(selection, "plan-commit", &store).unwrap();
    let runtime = guard_runtime::Runtime::new_v2(
        &registered,
        &sdk.contract,
        store.clone(),
        &project,
        sdk.retain().unwrap(),
        &scratch,
    )
    .unwrap();
    let (_, completion) = runtime
        .execute_v2_with_receipt(
            &id("OB-1"),
            "external-source-run",
            guard_runtime::NativeVerifierCommandV2 {
                verifier_id: id("java-package-check"),
                command: native_command.clone(),
            },
            guard_runner::Cancellation::new(),
        )
        .await
        .unwrap()
        .into_parts();
    let completion = completion.unwrap();
    let provenance = completion.provenance();
    let source_pair = Ed25519KeyPair::from_seed_unchecked(&[101; 32]).unwrap();
    host["producers"] = json!([{"issuer":"source-controller","keyId":"source-key","publicKey":STANDARD.encode(source_pair.public_key().as_ref()),"revocationAuthority":"source-revocation-authority","revocationRef":"source-current","audience":"controller-1","repositoryId":"repo-example","taskId":"REQ-1","contractDigest":sdk.contract.digest(),"policyDigest":sdk.package.digest(),"scopeDigest":provenance.scope.approved_scope_digest,"producer":provenance.producer,"domain":provenance.domain,"verifierPinDigest":provenance.verifier_pin_digest,"profile":provenance.profile,"approvedInvocation":provenance.approved_invocation,"allowedSubjectKinds":["commit"],"maxValidityMs":3600000}]);
    host["revocations"]["SOURCE-1"] = json!(format!("{prefix}/source-revocation.json"));
    write(&root.join("host.json"), &host);
    let now = chrono::Utc::now();
    let expires = guard_protocol::UtcTimestamp::new(
        (now + chrono::Duration::minutes(30)).to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
    )
    .unwrap();
    let revocation = json!({"authority":"source-revocation-authority","issuer":"source-controller","keyId":"source-key","receiptId":"SOURCE-1","revocationRef":"source-current","revision":1,"stateDigest":hash(b"source-current-active"),"validFrom":(now-chrono::Duration::minutes(1)).to_rfc3339_opts(chrono::SecondsFormat::Millis,true),"expiresAt":expires,"revoked":false});
    write(&trust.path().join("source-revocation.json"), &revocation);
    let source_ports = Arc::new(file_ports::FilePorts(
        host_config::HostConfig::load().unwrap(),
    ));
    let source_verifier = guard_authority::ProducerVerifier::new(
        source_ports.clone(),
        source_ports.clone(),
        source_ports,
    );
    let signing = source_verifier
        .prepare(&completion, id("SOURCE-1"), id("controller-1"), expires)
        .unwrap();
    let raw = signing
        .complete(source_pair.sign(&signing.signing_bytes().unwrap()).as_ref())
        .unwrap();
    let attestation = temporary.path().join("source-attestation.json");
    fs::write(&attestation, &raw).unwrap();
    let verified = expect(
        0,
        &[
            "evidence",
            "source-verify",
            "--selection",
            selection,
            "--plan-operation",
            "plan-commit",
            "--file",
            attestation.to_str().unwrap(),
        ],
    );
    assert_eq!(
        verified["result"]["source"],
        "independently_verified_current"
    );
    assert_eq!(verified["quality"], "not_evaluated");
    let mut fake_source: Value = serde_json::from_slice(&raw).unwrap();
    fake_source["provenance"]["exitCode"] = json!(7);
    write(&attestation, &fake_source);
    expect(
        1,
        &[
            "evidence",
            "source-verify",
            "--selection",
            selection,
            "--plan-operation",
            "plan-commit",
            "--file",
            attestation.to_str().unwrap(),
        ],
    );
    fs::write(&attestation, &raw).unwrap();
    let mut revoked = revocation.clone();
    revoked["revoked"] = json!(true);
    revoked["revision"] = json!(2);
    revoked["stateDigest"] = json!(hash(b"source-revoked"));
    write(&trust.path().join("source-revocation.json"), &revoked);
    expect(
        1,
        &[
            "evidence",
            "source-verify",
            "--selection",
            selection,
            "--plan-operation",
            "plan-commit",
            "--file",
            attestation.to_str().unwrap(),
        ],
    );
    write(&trust.path().join("source-revocation.json"), &revocation);
    // Git配置变化不会因保留旧环境声明而复用；更新显式环境后旧批准config仍不匹配。
    let mut changed = host.clone();
    changed["git"]["environment"]["LANG"] = json!("C");
    write(&root.join("host.json"), &changed);
    let refused = expect(
        3,
        &[
            "plan",
            "restore",
            "--selection",
            selection,
            "--operation",
            "plan-commit",
        ],
    );
    assert!(
        refused
            .to_string()
            .contains("does not bind actual Git provider")
    );
    let updated_git = prepared::git_identity(&host_config::HostConfig::load().unwrap()).unwrap();
    changed["verifiers"][0]["environment"]["GUARDCORE_GIT_PROVIDER_DIGEST"] = json!(updated_git);
    write(&root.join("host.json"), &changed);
    expect(
        3,
        &[
            "plan",
            "restore",
            "--selection",
            selection,
            "--operation",
            "plan-commit",
        ],
    );
    write(&root.join("host.json"), &host);
    let mut overlap = host.clone();
    overlap["stateRoot"] = overlap["scratchRoot"].clone();
    write(&root.join("host.json"), &overlap);
    let refused = expect(3, &["doctor"]);
    assert!(refused.to_string().contains("must be disjoint"));
    write(&root.join("host.json"), &host);
    let bad = temporary.path().join("malformed-selection.json");
    fs::write(&bad, b"{").unwrap();
    expect(
        2,
        &["plan", "preview", "--selection", bad.to_str().unwrap()],
    );
    // 先观察原生进程已经flush，再提交精确取消请求，不能把尚未派发误认为丢失输出。
    fs::write(project.join("src/Example.java"), b"TIMEOUT").unwrap();
    let worktree = temporary.path().join("worktree-selection.json");
    write(&worktree, &json!({"kind":"worktree","base":base}));
    let worktree = worktree.to_str().unwrap();
    expect(
        0,
        &[
            "plan",
            "register",
            "--selection",
            worktree,
            "--operation",
            "plan-worktree",
        ],
    );
    let request_file = scratch.join("cancel.json");
    let count = fs::read_to_string(&marker).unwrap().lines().count();
    let mut child = Command::new(env!("CARGO_BIN_EXE_guardcore"))
        .args([
            "run",
            "--selection",
            worktree,
            "--plan-operation",
            "plan-worktree",
            "--operation",
            "native-cancel",
            "--obligation",
            "OB-1",
            "--cancel-request",
            request_file.to_str().unwrap(),
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(90);
    loop {
        if fs::read_to_string(&marker)
            .unwrap_or_default()
            .lines()
            .count()
            > count
        {
            break;
        }
        assert!(
            child.try_wait().unwrap().is_none(),
            "execution must reach flush marker before cancellation"
        );
        assert!(
            std::time::Instant::now() < deadline,
            "bounded fixture marker wait"
        );
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    let pending = scratch.join("request.pending");
    write(
        &pending,
        &json!({"schemaVersion":"guard.cli-cancel/v1","repositoryId":"repo-example","taskId":"REQ-1","operationId":"native-cancel"}),
    );
    fs::rename(&pending, &request_file).unwrap();
    let (code, canceled) = parse_output(child.wait_with_output().unwrap());
    assert_eq!(code, 130, "{canceled}");
    assert_eq!(
        canceled["result"]["observation"]["executionStatus"],
        "CANCELLED"
    );
    let operation = canceled["result"]["observation"]["stdout"]["operation"]
        .as_str()
        .unwrap();
    let raw = expect(0, &["evidence", "show", "--operation", operation, "--raw"]);
    let bytes = STANDARD
        .decode(raw["result"]["privateRaw"]["bytes"].as_str().unwrap())
        .unwrap();
    assert!(String::from_utf8_lossy(&bytes).contains("OBSERVED=TIMEOUT"));
    let count = fs::read_to_string(&marker).unwrap().lines().count();
    // 旧操作的取消消息不得扩大到另一操作；坏请求必须失败，不能静默忽略。
    let invalid = expect(
        2,
        &[
            "run",
            "--selection",
            worktree,
            "--plan-operation",
            "plan-worktree",
            "--operation",
            "wrong-cancel-scope",
            "--obligation",
            "OB-1",
            "--cancel-request",
            request_file.to_str().unwrap(),
        ],
    );
    assert!(
        invalid
            .to_string()
            .contains("scope/version/operation mismatch")
    );
    assert_eq!(fs::read_to_string(&marker).unwrap().lines().count(), count);
    let timed = expect(
        3,
        &[
            "run",
            "--selection",
            worktree,
            "--plan-operation",
            "plan-worktree",
            "--operation",
            "native-timeout",
            "--obligation",
            "OB-1",
        ],
    );
    assert_eq!(
        timed["result"]["observation"]["executionStatus"],
        "TIMED_OUT"
    );
    assert!(fs::read_to_string(&marker).unwrap().lines().count() > count);
    let operation = timed["result"]["observation"]["stdout"]["operation"]
        .as_str()
        .unwrap();
    let raw = expect(0, &["evidence", "show", "--operation", operation, "--raw"]);
    let bytes = STANDARD
        .decode(raw["result"]["privateRaw"]["bytes"].as_str().unwrap())
        .unwrap();
    assert!(String::from_utf8_lossy(&bytes).contains("OBSERVED=TIMEOUT"));
    fs::write(
        temporary.path().join("state/evidence.sqlite3"),
        b"not a SQLite database",
    )
    .unwrap();
    expect(4, &["evidence", "verify", "--operation", op]);
}
