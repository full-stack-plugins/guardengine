use guard_protocol::{ProtocolDocument, TrustLevel, decode, encode};
use serde_json::{Value, json};

fn release() -> Value {
    let digest = format!("sha256:{}", "a".repeat(64));
    json!({"documentType":"SubjectSnapshot","schemaVersion":"1.0.0","kind":"release_artifact",
        "repositoryId":"repo-release","inputManifestDigest":digest,"extensions":{},
        "artifact":{"name":"example-1.0.jar","contentDigest":digest,"size":123,"mediaType":"application/java-archive"},
        "source":{"objectFormat":"sha1","commit":"a".repeat(40),"tree":"b".repeat(40),"inputManifestDigest":digest},
        "build":{"buildId":"ci-run-123-attempt-1","definitionDigest":digest,"invocationDigest":digest,"inputManifestDigest":digest,
            "producer":{"schemaVersion":"1.0.0","producerId":"ci-builder","version":"1.0.0","artifactDigest":digest,"configDigest":digest,"adapterVersion":"1.0.0","executionEnvironmentDigest":digest,"extensions":{}}}})
}
#[test]
fn release_artifact_is_a_distinct_closed_candidate_not_a_git_commit_or_trusted_proof() {
    let value = release();
    let decoded = decode(&serde_json::to_vec(&value).unwrap())
        .expect("D06 release artifact candidate missing");
    assert_eq!(decoded.trust_level(), TrustLevel::Unverified);
    let roundtrip = encode(decoded.document()).unwrap();
    assert_eq!(serde_json::from_slice::<Value>(&roundtrip).unwrap(), value);
    assert!(matches!(
        decoded.document(),
        ProtocolDocument::SubjectSnapshot(_)
    ));
    let ProtocolDocument::SubjectSnapshot(subject) = decoded.document() else {
        panic!("subject expected")
    };
    assert!(subject.as_git().is_none());
    assert_eq!(
        subject
            .as_release_artifact()
            .unwrap()
            .artifact
            .name
            .as_str(),
        "example-1.0.jar"
    );
}
#[test]
fn release_requires_exact_artifact_build_and_source_fields_and_rejects_git_relabelling() {
    let valid = release();
    for pointer in [
        "/artifact/contentDigest",
        "/artifact/size",
        "/artifact/mediaType",
        "/source/commit",
        "/source/tree",
        "/source/objectFormat",
        "/build/buildId",
        "/build/producer",
        "/build/definitionDigest",
        "/build/invocationDigest",
        "/build/inputManifestDigest",
    ] {
        let mut wrong = valid.clone();
        let (parent, name) = pointer.rsplit_once('/').unwrap();
        wrong
            .pointer_mut(parent)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .remove(name);
        assert!(
            decode(&serde_json::to_vec(&wrong).unwrap()).is_err(),
            "accepted missing {pointer}"
        );
    }
    for (field, value) in [
        ("head", json!("a".repeat(40))),
        ("tree", json!("b".repeat(40))),
        ("base", json!("c".repeat(40))),
        ("worktreeId", json!("wt")),
        ("targetRef", json!("refs/heads/main")),
        ("objectFormat", json!("sha1")),
    ] {
        let mut wrong = valid.clone();
        wrong[field] = value;
        assert!(
            decode(&serde_json::to_vec(&wrong).unwrap()).is_err(),
            "release accepted Git field {field}"
        );
    }
    let mut masquerade: Value =
        serde_json::from_slice(include_bytes!("../fixtures/valid/subject-commit.json")).unwrap();
    masquerade["kind"] = json!("release_artifact");
    assert!(decode(&serde_json::to_vec(&masquerade).unwrap()).is_err());
}
#[test]
fn release_oid_size_and_descriptor_validation_is_strict() {
    for bad_size in [
        json!(-1),
        json!(1.5),
        json!(9007199254740992u64),
        json!("123"),
    ] {
        let mut wrong = release();
        wrong["artifact"]["size"] = bad_size;
        assert!(decode(&serde_json::to_vec(&wrong).unwrap()).is_err());
    }
    for name in [
        "../secret",
        "dir/file.jar",
        "C:\\file.jar",
        "/tmp/file",
        "a\n.jar",
        "a..jar",
        "NUL",
        "con.txt",
        "trail.",
    ] {
        let mut wrong = release();
        wrong["artifact"]["name"] = json!(name);
        assert!(
            decode(&serde_json::to_vec(&wrong).unwrap()).is_err(),
            "unsafe artifact name {name}"
        );
    }
    for media in [
        "text",
        "application/java-archive; charset=utf-8",
        "text/plain\r\nX: evil",
        "Text/Plain",
        "a//b",
    ] {
        let mut wrong = release();
        wrong["artifact"]["mediaType"] = json!(media);
        assert!(decode(&serde_json::to_vec(&wrong).unwrap()).is_err());
    }
    let mut wrong = release();
    wrong["source"]["objectFormat"] = json!("sha256");
    assert!(decode(&serde_json::to_vec(&wrong).unwrap()).is_err());
    wrong["source"]["commit"] = json!("a".repeat(64));
    wrong["source"]["tree"] = json!("b".repeat(64));
    assert!(decode(&serde_json::to_vec(&wrong).unwrap()).is_ok());
}

#[test]
fn four_existing_git_wires_round_trip_and_do_not_become_release_artifacts() {
    for bytes in [
        include_bytes!("../fixtures/valid/subject-worktree.json").as_slice(),
        include_bytes!("../fixtures/valid/subject-index.json"),
        include_bytes!("../fixtures/valid/subject-commit.json"),
        include_bytes!("../fixtures/valid/subject-snapshot.json"),
    ] {
        let decoded = decode(bytes).unwrap();
        let ProtocolDocument::SubjectSnapshot(subject) = decoded.document() else {
            panic!("subject expected")
        };
        assert!(subject.as_git().is_some());
        assert!(subject.as_release_artifact().is_none());
        assert_eq!(
            encode(decoded.document()).unwrap(),
            guard_protocol::canonicalize(bytes).unwrap()
        );
    }
}

#[test]
fn public_release_mutation_and_duplicate_raw_fields_do_not_bypass_strict_entry() {
    let bytes = serde_json::to_vec(&release()).unwrap();
    let mut document = decode(&bytes).unwrap().document().clone();
    let ProtocolDocument::SubjectSnapshot(guard_protocol::SubjectSnapshot::ReleaseArtifact(
        subject,
    )) = &mut document
    else {
        panic!("release expected")
    };
    subject.artifact.size = 9007199254740992;
    assert!(encode(&document).is_err());
    let duplicate = String::from_utf8(bytes)
        .unwrap()
        .replace("\"size\":123", "\"size\":123,\"size\":123");
    assert!(decode(duplicate.as_bytes()).is_err());
}
