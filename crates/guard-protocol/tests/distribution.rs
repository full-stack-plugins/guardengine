use guard_protocol::{ProtocolDocument, canonicalize, decode, encode};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{fs, path::Path};

#[test]
fn frozen_canonical_vectors_are_byte_exact() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let vectors: Vec<Value> =
        serde_json::from_slice(&fs::read(root.join("fixtures/canonical-vectors.json")).unwrap())
            .unwrap();
    for vector in vectors {
        assert_eq!(
            canonicalize(vector["input"].as_str().unwrap().as_bytes()).unwrap(),
            vector["canonical"].as_str().unwrap().as_bytes()
        );
    }
}

#[test]
fn every_required_root_field_is_really_required() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let schema: Value =
        serde_json::from_slice(&fs::read(root.join("schemas/engineering.schema.json")).unwrap())
            .unwrap();
    let cases: Vec<Value> =
        serde_json::from_slice(&fs::read(root.join("fixtures/cases.json")).unwrap()).unwrap();
    for case in cases.into_iter().filter(|v| v["valid"] == true) {
        let value: Value = serde_json::from_slice(
            &fs::read(root.join("fixtures").join(case["file"].as_str().unwrap())).unwrap(),
        )
        .unwrap();
        let name = format!("{}Document", value["documentType"].as_str().unwrap());
        for field in schema["$defs"][&name]["required"].as_array().unwrap() {
            let mut missing = value.clone();
            missing
                .as_object_mut()
                .unwrap()
                .remove(field.as_str().unwrap());
            assert!(
                decode(&serde_json::to_vec(&missing).unwrap()).is_err(),
                "{} accepted missing {field}",
                case["file"]
            );
        }
    }
}

#[test]
fn public_typed_mutation_cannot_bypass_protocol_integer_identity() {
    let mut doc = decode(include_bytes!("../fixtures/valid/rule-definition.json"))
        .unwrap()
        .document()
        .clone();
    if let ProtocolDocument::RuleDefinition(rule) = &mut doc {
        rule.parameters = serde_json::json!({"number":9007199254740993u64});
    } else {
        panic!("rule fixture")
    }
    assert!(encode(&doc).is_err());
}

#[test]
fn input_budgets_reject_oversize_depth_and_collections() {
    let too_large = vec![b' '; 2 * 1024 * 1024 + 1];
    assert!(canonicalize(&too_large).is_err());
    let deep = format!("{}0{}", "[".repeat(66), "]".repeat(66));
    assert!(canonicalize(deep.as_bytes()).is_err());
    let array = format!("[{}]", vec!["0"; 8193].join(","));
    assert!(canonicalize(array.as_bytes()).is_err());
    let text = serde_json::to_vec(&"x".repeat(65537)).unwrap();
    assert!(canonicalize(&text).is_err());
    for number in ["NaN", "Infinity", "-Infinity", "1e999", "-1e999"] {
        assert!(canonicalize(number.as_bytes()).is_err(), "{number}");
    }
}

#[test]
fn release_manifest_freezes_schemas_and_golden_fixture_bytes() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let manifest: Value =
        serde_json::from_slice(&fs::read(root.join("release-manifest.json")).unwrap()).unwrap();
    assert_eq!(manifest["status"], "candidate-unpublished");
    for file in manifest["files"].as_array().unwrap() {
        let path = file["path"].as_str().unwrap();
        assert!(!path.starts_with('/') && !path.contains(".."));
        let bytes = fs::read(root.join(path)).unwrap();
        assert!(
            !bytes.windows(2).any(|pair| pair == b"\r\n"),
            "locked text must use LF so Git checkouts retain its raw digest: {path}"
        );
        assert_eq!(
            format!("sha256:{:x}", Sha256::digest(bytes)),
            file["digest"].as_str().unwrap(),
            "{path}"
        );
    }
}
