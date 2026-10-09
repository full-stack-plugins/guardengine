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
fn load(v: &Value) -> bool {
    load_envelope_json(&serde_json::to_vec(v).unwrap(), EvidenceProfile::NativeOnly).is_ok()
}
#[test]
fn required_fields_and_profiles() {
    let v = envelope();
    assert!(load(&v));
    assert!(
        load_envelope_json(
            &serde_json::to_vec(&v).unwrap(),
            EvidenceProfile::EngineBacked
        )
        .is_err()
    );
    for field in v.as_object().unwrap().keys() {
        let mut bad = v.clone();
        bad.as_object_mut().unwrap().remove(field);
        assert!(!load(&bad), "missing {field}");
    }
    for field in ["mergeGroupId", "baselineDigest"] {
        let mut bad = v.clone();
        bad["binding"].as_object_mut().unwrap().remove(field);
        assert!(!load(&bad), "missing {field}");
    }
}
#[test]
fn rejects_unknown_duplicate_and_unsupported_input() {
    let mut v = envelope();
    v["binding"]["trusted"] = json!(true);
    assert!(!load(&v));
    let mut v = envelope();
    v["apiVersion"] = json!("guard.integration/v2");
    assert!(!load(&v));
    let raw =
        serde_json::to_string(&envelope())
            .unwrap()
            .replacen("{", "{\"runId\":\"duplicate\",", 1);
    assert!(load_envelope_json(raw.as_bytes(), EvidenceProfile::NativeOnly).is_err());
    assert!(load_envelope_json(&vec![b' '; 1_048_577], EvidenceProfile::NativeOnly).is_err());
}
#[test]
fn validates_binding_coverage_lifecycle_and_time() {
    for (pointer, value) in [
        ("/binding/candidateOid", json!("branch")),
        ("/binding/requirementIds", json!(["R2", "R1"])),
        ("/binding/sourceSnapshotDigest", json!("sha256:bad")),
        ("/coverage/missingScopes", json!(["lint"])),
        ("/coverage/observedScopes", json!([])),
        ("/decision", Value::Null),
        ("/finishedAt", json!("2026-10-08T10:00:00Z")),
        ("/startedAt", json!("not-a-time")),
        ("/expiresAt", json!("2026-10-08T10:00:00Z")),
        ("/producer/guard", json!(" ")),
        ("/artifacts/domain/0/uri", json!("file:///etc/passwd")),
        (
            "/artifacts/domain/0/uri",
            json!("artifact://local/../secret"),
        ),
    ] {
        let mut v = envelope();
        *v.pointer_mut(pointer).unwrap() = value;
        assert!(!load(&v), "accepted {pointer}: {v}");
    }
    let mut v = envelope();
    v["coverage"]["status"] = json!("partial");
    v["coverage"]["observedScopes"] = json!([]);
    v["coverage"]["missingScopes"] = json!(["lint"]);
    assert!(!load(&v));
    v["decision"] = json!("BLOCK");
    assert!(load(&v));
    let mut v = envelope();
    v["runStatus"] = json!("error");
    assert!(!load(&v));
    v["decision"] = Value::Null;
    v["diagnostics"] =
        json!([{"code":"runner.failed","message":"failed","retryable":false,"source":null}]);
    assert!(load(&v));
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
#[test]
fn artifact_verification_checks_exact_bytes_recomputation_and_binding() {
    use guardengine::integration::verify_engine_artifacts;
    let (v, cb, fb, rb) = artifacts();
    let e = load_envelope_json(
        &serde_json::to_vec(&v).unwrap(),
        EvidenceProfile::EngineBacked,
    )
    .unwrap();
    assert!(verify_engine_artifacts(&e, &cb, &fb, &rb).is_ok());
    let mut altered = rb.clone();
    altered.push(b' ');
    assert!(verify_engine_artifacts(&e, &cb, &fb, &altered).is_err());
    let mut bad = e.clone();
    bad.producer.analyzer_version = "2".into();
    assert!(verify_engine_artifacts(&bad, &cb, &fb, &rb).is_err());
    let mut bad = e.clone();
    bad.binding.source_snapshot_digest = format!("sha256:{}", "f".repeat(64));
    assert!(verify_engine_artifacts(&bad, &cb, &fb, &rb).is_err());
    let mut bad = e.clone();
    bad.decision = Some(guardengine::Decision::Block);
    assert!(verify_engine_artifacts(&bad, &cb, &fb, &rb).is_err());
    use sha2::{Digest, Sha256};
    let mut forged: Value = serde_json::from_slice(&rb).unwrap();
    forged["signed"] = json!(true);
    let forged = serde_json::to_vec(&forged).unwrap();
    let mut bad = e;
    bad.artifacts.report.as_mut().unwrap().digest = format!("sha256:{:x}", Sha256::digest(&forged));
    assert!(verify_engine_artifacts(&bad, &cb, &fb, &forged).is_err());
}
#[test]
fn golden_vectors_are_repeatable() {
    use sha2::{Digest, Sha256};
    let input = include_bytes!("fixtures/integration-envelope/valid-native.json");
    let expected = include_bytes!("fixtures/integration-envelope/native.canonical");
    for _ in 0..3 {
        let e = load_envelope_json(input, EvidenceProfile::NativeOnly).unwrap();
        let actual = serde_json::to_vec(&e).unwrap();
        assert_eq!(actual, expected);
        assert_eq!(
            format!("sha256:{:x}", Sha256::digest(&actual)),
            include_str!("fixtures/integration-envelope/native.sha256").trim()
        );
    }
}
#[test]
fn closed_schema_rejects_extensions() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/integration-envelope");
    for entry in std::fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|x| x == "json") {
            let expected = !path
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with("invalid-");
            assert_eq!(
                load_envelope_json(&std::fs::read(&path).unwrap(), EvidenceProfile::NativeOnly)
                    .is_ok(),
                expected,
                "{}",
                path.display()
            );
        }
    }
}
