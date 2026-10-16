use serde_json::json;
use std::fs;

#[test]
fn signed_payload_requires_pinned_key_and_rejects_tampering() {
    let dir = tempfile::tempdir().unwrap();
    let private = dir.path().join("private.pk8");
    let public = dir.path().join("public.hex");
    guard_evidence::keygen(&private, &public).unwrap();
    let payload =
        json!({"schema_version":"testguard.execution/v1","status":"PASS","nested":{"b":2,"a":1}});
    let signed = guard_evidence::sign_receipt(&payload, &private).unwrap();
    assert_eq!(
        guard_evidence::verify_receipt(&signed, &public).unwrap(),
        payload
    );
    let mut tampered = signed.clone();
    tampered["payload"]["status"] = json!("FAIL");
    assert!(guard_evidence::verify_receipt(&tampered, &public).is_err());
    let other = dir.path().join("other.hex");
    guard_evidence::keygen(&dir.path().join("other.pk8"), &other).unwrap();
    assert!(guard_evidence::verify_receipt(&signed, &other).is_err());
    tampered = signed.clone();
    tampered["signature"]["algorithm"] = json!("none");
    assert!(guard_evidence::verify_receipt(&tampered, &public).is_err());
    assert!(guard_evidence::verify_receipt(&payload, &public).is_err());
}

#[test]
fn snapshots_bind_content_and_inventory_but_exclude_declared_build_outputs() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("pom.xml"), "<project/>").unwrap();
    let before = guard_evidence::snapshot_project(dir.path()).unwrap();
    fs::create_dir(dir.path().join("target")).unwrap();
    fs::write(dir.path().join("target/result"), "build").unwrap();
    assert_eq!(
        before,
        guard_evidence::snapshot_project(dir.path()).unwrap()
    );
    fs::write(dir.path().join("source.java"), "class Example {}").unwrap();
    assert_ne!(
        before["digest"],
        guard_evidence::snapshot_project(dir.path()).unwrap()["digest"]
    );
    fs::remove_file(dir.path().join("source.java")).unwrap();
    assert_eq!(
        before,
        guard_evidence::snapshot_project(dir.path()).unwrap()
    );
    fs::write(dir.path().join("pom.xml"), "<changed/>").unwrap();
    assert_ne!(
        before["digest"],
        guard_evidence::snapshot_project(dir.path()).unwrap()["digest"]
    );
}

#[test]
fn keys_are_never_overwritten_and_digest_is_key_order_independent() {
    let dir = tempfile::tempdir().unwrap();
    let private = dir.path().join("key");
    let public = dir.path().join("pub");
    guard_evidence::keygen(&private, &public).unwrap();
    let bytes = fs::read(&private).unwrap();
    assert!(guard_evidence::keygen(&private, &public).is_err());
    assert_eq!(fs::read(private).unwrap(), bytes);
    assert_eq!(
        guard_evidence::digest_json(&json!({"a":1,"b":2})).unwrap(),
        guard_evidence::digest_json(&json!({"b":2,"a":1})).unwrap()
    );
}

#[test]
fn oversized_source_is_not_a_complete_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("oversized"), vec![0; 4 * 1024 * 1024 + 1]).unwrap();
    assert!(guard_evidence::snapshot_project(dir.path()).is_err());
}

#[test]
fn execution_verification_rejects_signed_but_inconsistent_claims() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("project");
    fs::create_dir(&root).unwrap();
    fs::write(root.join("source"), "candidate").unwrap();
    let private = dir.path().join("key");
    let public = dir.path().join("pub");
    guard_evidence::keygen(&private, &public).unwrap();
    let artifact = dir.path().join("report.xml");
    fs::write(&artifact, "fresh report").unwrap();
    let executable = dir.path().join("runner");
    fs::write(&executable, "explicit trusted runner").unwrap();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    let plan = guard_evidence::digest_json(&json!({"required_tests":["Example#works"]})).unwrap();
    let payload = json!({"schema_version":"testguard.execution/v1","producer":"testguard","project":root.canonicalize().unwrap(),"snapshot":guard_evidence::snapshot_project(&root).unwrap(),"plan_digest":plan,"runner":"maven","executable":{"path":executable,"sha256":guard_evidence::hash_file(&executable).unwrap()},"argv":["test"],"started_at_unix_ms":now-100,"finished_at_unix_ms":now,"exit_code":0,"status":"PASS","execution_verified":true,"required_tests":["Example#works"],"observed_tests":["Example#works"],"artifacts":[{"path":artifact,"sha256":guard_evidence::hash_file(&artifact).unwrap()}]});
    let signed = guard_evidence::sign_receipt(&payload, &private).unwrap();
    guard_evidence::verify_execution(&signed, &public, &root, &plan, 60).unwrap();
    for (field, value) in [
        ("producer", json!("fake")),
        ("status", json!("FAIL")),
        ("exit_code", json!(1)),
        ("execution_verified", json!(false)),
        ("observed_tests", json!([])),
        ("required_tests", json!([])),
        ("finished_at_unix_ms", json!(now + 60000)),
        ("runner", json!("shell")),
        ("artifacts", json!([])),
    ] {
        let mut invalid = payload.clone();
        invalid[field] = value;
        let signed = guard_evidence::sign_receipt(&invalid, &private).unwrap();
        assert!(
            guard_evidence::verify_execution(&signed, &public, &root, &plan, 60).is_err(),
            "accepted invalid {field}"
        );
    }
    fs::write(&artifact, "changed report").unwrap();
    assert!(guard_evidence::verify_execution(&signed, &public, &root, &plan, 60).is_err());
    fs::write(&artifact, "fresh report").unwrap();
    fs::write(root.join("source"), "changed candidate").unwrap();
    assert!(guard_evidence::verify_execution(&signed, &public, &root, &plan, 60).is_err());
}
