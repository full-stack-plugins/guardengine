use guardengine::integration::{
    EvidenceProfile, GuardRunEnvelope, load_envelope_json, verify_engine_artifacts,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, process::Command};
fn invoke(
    command: &str,
    contract: &std::path::Path,
    facts: &std::path::Path,
    report: Option<&std::path::Path>,
) -> std::process::Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_guardengine"));
    cmd.arg(command)
        .arg("--contract")
        .arg(contract)
        .arg("--facts")
        .arg(facts);
    if let Some(p) = report {
        cmd.arg("--report").arg(p);
    }
    cmd.output().unwrap()
}
#[test]
fn native_cli_and_opt_in_adapter_preserve_neutral_rule_matrix() {
    for (enforcement, matched, partial, exit, decision) in [
        ("enforce", false, false, 0, "ALLOW"),
        ("enforce", true, false, 2, "BLOCK"),
        ("review", true, false, 3, "REQUIRE_APPROVAL"),
        ("advise", true, false, 0, "ALLOW"),
        ("review", true, true, 2, "BLOCK"),
    ] {
        let contract = json!({"apiVersion":guardengine::API_VERSION,"kind":"GuardContract","metadata":{"id":"p","revision":"1"},"spec":{"rules":[{"id":"r","description":"parity","enforcement":enforcement,"assertion":{"type":"forbid_relation","subject":"a","predicate":"depends_on","object":"b"}}]}});
        let facts = json!({"apiVersion":guardengine::API_VERSION,"kind":"GuardFacts","analyzer":{"id":"native","version":"1"},"subject":{"id":"repo","snapshotDigest":format!("sha256:{}","c".repeat(64))},"completeness":if partial{"partial"}else{"complete"},"facts":if matched{json!([{"subject":"a","predicate":"depends_on","object":"b","source":"source.txt:1"}])}else{json!([])},"diagnostics":if partial{json!(["scope missing"])}else{json!([])}});
        let cb = serde_json::to_vec(&contract).unwrap();
        let fb = serde_json::to_vec(&facts).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let cp = dir.path().join("contract.yaml");
        let fp = dir.path().join("facts.json");
        let rp = dir.path().join("report.json");
        fs::write(&cp, &cb).unwrap();
        fs::write(&fp, &fb).unwrap();
        let stdout = invoke("evaluate", &cp, &fp, None);
        assert_eq!(stdout.status.code(), Some(exit), "{enforcement}/{partial}");
        let report: Value = serde_json::from_slice(&stdout.stdout).unwrap();
        assert_eq!(report["decision"], decision);
        let file = invoke("evaluate", &cp, &fp, Some(&rp));
        assert_eq!(file.status.code(), Some(exit));
        assert!(file.stdout.is_empty());
        assert_eq!(
            serde_json::from_slice::<Value>(&fs::read(&rp).unwrap()).unwrap(),
            report
        );
        let verify = invoke("verify", &cp, &fp, Some(&rp));
        assert_eq!(verify.status.code(), Some(exit));
        assert!(verify.stdout.is_empty());
        assert!(!verify.stderr.is_empty());
        let mut envelope: Value = serde_json::from_slice(include_bytes!(
            "fixtures/integration-envelope/valid-engine.json"
        ))
        .unwrap();
        envelope["decision"] = json!(decision);
        if partial {
            envelope["coverage"]["status"] = json!("partial");
            envelope["coverage"]["observedScopes"] = json!([]);
            envelope["coverage"]["missingScopes"] = json!(["lint"]);
        }
        for (kind, bytes) in [
            ("contract", &cb),
            ("facts", &fb),
            ("report", &stdout.stdout),
        ] {
            envelope["artifacts"][kind]["digest"] =
                json!(format!("sha256:{:x}", Sha256::digest(bytes)));
        }
        let e: GuardRunEnvelope = load_envelope_json(
            &serde_json::to_vec(&envelope).unwrap(),
            EvidenceProfile::EngineBacked,
        )
        .unwrap();
        let verified = verify_engine_artifacts(&e, &cb, &fb, &stdout.stdout).unwrap();
        let native = guardengine::evaluate(
            &guardengine::load_contract_yaml(&cb).unwrap(),
            &guardengine::load_facts_json(&fb).unwrap(),
        )
        .unwrap();
        assert_eq!(verified, native);
        if matched {
            assert_eq!(
                verified.evaluations[0].matched_facts[0].source,
                "source.txt:1"
            );
        }
        if enforcement == "advise" {
            assert_eq!(
                verified.evaluations[0].status,
                guardengine::RuleStatus::Pass
            );
            assert!(!verified.evaluations[0].matched_facts.is_empty());
        }
    }
}
#[test]
fn invalid_native_input_has_execution_exit_and_no_stdout_report() {
    let dir = tempfile::tempdir().unwrap();
    let cp = dir.path().join("contract.yaml");
    let fp = dir.path().join("facts.json");
    fs::write(&cp, b"apiVersion: unsupported").unwrap();
    fs::write(&fp, b"{}").unwrap();
    let result = invoke("evaluate", &cp, &fp, None);
    assert_eq!(result.status.code(), Some(4));
    assert!(result.stdout.is_empty());
    assert!(!result.stderr.is_empty());
}
