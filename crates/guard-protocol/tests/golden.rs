use guard_protocol::{GitPath, ProtocolDocument, TrustLevel, decode, encode};
use serde_json::Value;
use std::{fs, path::Path};

#[test]
fn golden_documents_preserve_unknowns_failures_and_authority_boundary() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures");
    let cases: Vec<Value> =
        serde_json::from_slice(&fs::read(root.join("cases.json")).unwrap()).unwrap();
    assert!(cases.len() >= 70);
    for case in cases {
        let bytes = fs::read(root.join(case["file"].as_str().unwrap())).unwrap();
        let decoded = decode(&bytes);
        if case["valid"] == true {
            let value = decoded.unwrap_or_else(|e| panic!("{}: {e}", case["file"]));
            assert_eq!(value.trust_level(), TrustLevel::Unverified);
            assert_eq!(value.original_bytes(), bytes);
            let canonical = encode(value.document()).unwrap();
            let roundtrip = decode(&canonical).unwrap();
            assert_eq!(roundtrip.document(), value.document());
            if let ProtocolDocument::GuardResult(r) = value.document() {
                if r.execution.status == guard_protocol::ExecutionStatus::TimedOut {
                    assert_eq!(r.assessment.verdict, guard_protocol::Verdict::Violation);
                    assert_eq!(r.assessment.finding_ids.len(), 1);
                }
            }
        } else {
            assert!(decoded.is_err(), "accepted invalid {}", case["file"]);
        }
    }
}

#[test]
fn raw_git_path_encoding_is_lossless_and_collision_free() {
    for bytes in [
        b"src/A.java".as_slice(),
        b"src/a.java",
        b"src/\xff.rs",
        "src/é.java".as_bytes(),
        "src/e\u{301}.java".as_bytes(),
    ] {
        let path = GitPath::from_bytes(bytes).unwrap();
        assert_eq!(path.to_bytes(), bytes);
        let encoded = serde_json::to_vec(&path).unwrap();
        let read: GitPath = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(read.to_bytes(), bytes);
    }
    assert_ne!(
        GitPath::from_bytes("é".as_bytes()).unwrap(),
        GitPath::from_bytes("e\u{301}".as_bytes()).unwrap()
    );
    for bytes in [b"../x".as_slice(), b"/x", b"a//b", b"a/./b", b"a\0b", b""] {
        assert!(GitPath::from_bytes(bytes).is_err());
    }
}

#[test]
fn every_distributed_schema_object_is_closed() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let schema: Value =
        serde_json::from_slice(&fs::read(root.join("schemas/engineering.schema.json")).unwrap())
            .unwrap();
    assert_eq!(schema["oneOf"].as_array().unwrap().len(), 11);
    for (name, definition) in schema["$defs"].as_object().unwrap() {
        if definition["type"] == "object" && name != "Extensions" {
            assert_eq!(definition["additionalProperties"], false, "{name}");
            let props = definition["properties"].as_object().unwrap();
            for required in definition["required"].as_array().unwrap() {
                assert!(props.contains_key(required.as_str().unwrap()), "{name}");
            }
        }
    }
}
