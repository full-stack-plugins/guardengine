use guardengine::integration::{EvidenceProfile, load_envelope_json};
use serde_json::{Value, json};
fn envelope() -> Value {
    json!({"apiVersion":"guard.integration/v1alpha1","kind":"GuardRunEnvelope","runId":"attempt-1",
 "producer":{"guard":"CodeGuard","version":"0.1.0","analyzerId":"native","analyzerVersion":"1"},
 "binding":{"repoId":"repo","taskId":"task","worktreeId":"worktree","requirementIds":["R1"],"candidateOid":"a".repeat(40),"baseOid":"b".repeat(40),"mergeGroupId":null,"sourceSnapshotDigest":format!("sha256:{}","c".repeat(64)),"baselineDigest":null},
 "runStatus":"completed","decision":"ALLOW","coverage":{"status":"complete","requiredScopes":["lint"],"observedScopes":["lint"],"missingScopes":[]},
 "artifacts":{"contract":null,"facts":null,"report":null,"domain":[{"uri":"artifact://local/native.json","digest":format!("sha256:{}","d".repeat(64)),"mediaType":"application/json"}]},
 "approvalRefs":[],"diagnostics":[],"startedAt":"2026-10-09T10:00:00Z","finishedAt":"2026-10-09T10:00:01Z","expiresAt":null})
}
fn artifacts() -> (Value, Vec<u8>, Vec<u8>, Vec<u8>) {
    use sha2::{Digest, Sha256};
    let contract = json!({"apiVersion":guardengine::API_VERSION,"kind":"GuardContract","metadata":{"id":"policy","revision":"1"},"spec":{"rules":[{"id":"deny","description":"test","enforcement":"enforce","assertion":{"type":"forbid_relation","subject":"a","predicate":"depends_on","object":"b"}}]}});
    let facts = json!({"apiVersion":guardengine::API_VERSION,"kind":"GuardFacts","analyzer":{"id":"native","version":"1"},"subject":{"id":"repo","snapshotDigest":format!("sha256:{}","c".repeat(64))},"completeness":"complete","facts":[],"diagnostics":[]});
    let cb = serde_json::to_vec(&contract).unwrap();
    let fb = serde_json::to_vec(&facts).unwrap();
    let report = guardengine::evaluate(
        &guardengine::load_contract_yaml(&cb).unwrap(),
        &guardengine::load_facts_json(&fb).unwrap(),
    )
    .unwrap();
    let rb = serde_json::to_vec(&report).unwrap();
    let mut v = envelope();
    v["artifacts"]["domain"] = json!([]);
    for (name, bytes) in [("contract", &cb), ("facts", &fb), ("report", &rb)] {
        v["artifacts"][name] = json!({"uri":format!("artifact://local/{name}"),"digest":format!("sha256:{:x}",Sha256::digest(bytes)),"mediaType":"application/json"});
    }
    (v, cb, fb, rb)
}

fn verify(
    yaml: &[u8],
    correct_report: bool,
) -> Result<guardengine::GuardReport, guardengine::integration::IntegrationError> {
    use sha2::{Digest, Sha256};
    let (mut value, _, facts, mut report) = artifacts();
    if correct_report {
        report = serde_json::to_vec(
            &guardengine::evaluate(
                &guardengine::load_contract_yaml(yaml).unwrap(),
                &guardengine::load_facts_json(&facts).unwrap(),
            )
            .unwrap(),
        )
        .unwrap();
    }
    value["artifacts"]["contract"]["digest"] = json!(format!("sha256:{:x}", Sha256::digest(yaml)));
    value["artifacts"]["report"]["digest"] = json!(format!("sha256:{:x}", Sha256::digest(&report)));
    let envelope = load_envelope_json(
        &serde_json::to_vec(&value).unwrap(),
        EvidenceProfile::EngineBacked,
    )
    .unwrap();
    guardengine::integration::verify_engine_artifacts(&envelope, yaml, &facts, &report)
}
fn contract(description: &str) -> String {
    format!(
        "apiVersion: guard.partme.ai/v1alpha1\nkind: GuardContract\nmetadata: {{id: policy, revision: '1'}}\nspec:\n  rules:\n    - id: deny\n      description: {description}\n      enforcement: enforce\n      assertion: {{type: forbid_relation, subject: a, predicate: depends_on, object: b}}\n"
    )
}
#[test]
fn verifier_rejects_scalar_alias_amplification_before_legacy_loading() {
    let mut yaml = String::from(
        "apiVersion: guard.partme.ai/v1alpha1\nkind: GuardContract\nmetadata: {id: policy, revision: '1'}\nspec:\n  rules:\n",
    );
    for i in 0..500 {
        yaml.push_str(&format!("    - id: r{i}\n      description: {}\n      enforcement: enforce\n      assertion: {{type: forbid_relation, subject: a, predicate: depends_on, object: b}}\n", if i == 0 { format!("&d {}", "x".repeat(131072)) } else { "*d".into() }));
    }
    assert_eq!(yaml.len(), 206573);
    let error = verify(yaml.as_bytes(), false).unwrap_err();
    assert!(
        error.0.contains("YAML anchors and aliases are unsupported"),
        "{error}"
    );
}
#[test]
fn verifier_accepts_literal_stars_and_ampersands() {
    for text in [
        "\"*alias &anchor\"",
        "'*alias &anchor'",
        "|\n        *alias &anchor",
        ">\n        *alias &anchor",
        "ordinary *star &ampersand",
    ] {
        assert!(verify(contract(text).as_bytes(), true).is_ok(), "{text}");
    }
}
#[test]
fn legacy_loader_keeps_anchor_support() {
    assert!(guardengine::load_contract_yaml(contract("&text ordinary").as_bytes()).is_ok());
}
#[test]
fn verifier_rejects_yaml_depth_before_native_schema_loading() {
    let yaml = format!("{}0{}", "[".repeat(65), "]".repeat(65));
    let error = verify(yaml.as_bytes(), false).unwrap_err();
    assert!(error.0.contains("YAML depth budget exceeded"), "{error}");
}
#[test]
fn verifier_rejects_yaml_event_count_before_native_schema_loading() {
    let yaml = format!("[{}]", "0,".repeat(65536));
    let error = verify(yaml.as_bytes(), false).unwrap_err();
    assert!(error.0.contains("YAML event budget exceeded"), "{error}");
}
#[test]
fn verifier_rejects_yaml_scalar_size_before_native_schema_loading() {
    let error = verify(contract(&"x".repeat(262145)).as_bytes(), false).unwrap_err();
    assert!(error.0.contains("YAML scalar budget exceeded"), "{error}");
}
#[test]
fn verifier_rejects_yaml_scalar_total_before_native_schema_loading() {
    let scalar = "x".repeat(262144);
    let yaml = format!("[{}]", vec![scalar; 33].join(","));
    let error = verify(yaml.as_bytes(), false).unwrap_err();
    assert!(error.0.contains("YAML scalar budget exceeded"), "{error}");
}
#[test]
fn verifier_rejects_explicit_tags_and_multiple_documents() {
    for (yaml, expected) in [
        (
            contract("!!str hello"),
            "YAML explicit tags are unsupported",
        ),
        (
            format!("{}---\nnull\n", contract("hello")),
            "YAML requires one document",
        ),
    ] {
        let error = verify(yaml.as_bytes(), false).unwrap_err();
        assert!(error.0.contains(expected), "{error}");
    }
}
