//! Frozen bytes from six producers. These tests establish interoperability,
//! never producer authentication or a domain capability absent from the manifest.
use guardengine::integration::{EvidenceProfile, load_envelope_json, verify_engine_artifacts};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

fn manifest() -> Value {
    serde_json::from_str(include_str!("../schemas/integration/consumer-matrix.json")).unwrap()
}

#[test]
fn frozen_six_consumer_bytes_verify_and_reject_mutations() {
    let manifest = manifest();
    let cases = manifest["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 16);
    for case in cases {
        let name = case["case"].as_str().unwrap();
        let directory =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(case["directory"].as_str().unwrap());
        for (file, hash) in case["fixtureSha256"].as_object().unwrap() {
            let bytes = fs::read(directory.join(file)).unwrap();
            assert_eq!(
                format!("{:x}", Sha256::digest(&bytes)),
                hash.as_str().unwrap(),
                "{name}/{file}"
            );
        }
        let raw = fs::read(directory.join("envelope.json")).unwrap();
        let native = case["evidenceProfile"] == "nativeOnly";
        let profile = if native {
            EvidenceProfile::NativeOnly
        } else {
            EvidenceProfile::EngineBacked
        };
        let envelope = load_envelope_json(&raw, profile).unwrap();
        let mut value: Value = serde_json::from_slice(&raw).unwrap();
        assert_eq!(value["decision"], case["decision"], "{name}");
        assert_eq!(value["coverage"]["status"], case["coverage"], "{name}");
        value["apiVersion"] = json!("guard.integration/v999");
        assert!(
            load_envelope_json(&serde_json::to_vec(&value).unwrap(), profile).is_err(),
            "{name}"
        );
        value = serde_json::from_slice(&raw).unwrap();
        value["unrecognizedCapability"] = json!(true);
        assert!(
            load_envelope_json(&serde_json::to_vec(&value).unwrap(), profile).is_err(),
            "{name}"
        );
        if native {
            assert!(load_envelope_json(&raw, EvidenceProfile::EngineBacked).is_err());
            let bytes = fs::read(directory.join("native.json")).unwrap();
            assert_eq!(
                envelope.artifacts.domain[0].digest,
                format!("sha256:{:x}", Sha256::digest(bytes))
            );
            continue;
        }
        let contract = fs::read(directory.join("contract.yaml")).unwrap();
        let facts = fs::read(directory.join("facts.json")).unwrap();
        let report = fs::read(directory.join("report.json")).unwrap();
        verify_engine_artifacts(&envelope, &contract, &facts, &report).unwrap();
        for target in 0..3 {
            let mut artifacts = [contract.clone(), facts.clone(), report.clone()];
            artifacts[target].push(b' ');
            assert!(
                verify_engine_artifacts(&envelope, &artifacts[0], &artifacts[1], &artifacts[2])
                    .is_err(),
                "{name}: tamper {target}"
            );
        }
        let mut wrong = envelope.clone();
        wrong.decision = Some(if envelope.decision == Some(guardengine::Decision::Allow) {
            guardengine::Decision::Block
        } else {
            guardengine::Decision::Allow
        });
        assert!(
            verify_engine_artifacts(&wrong, &contract, &facts, &report).is_err(),
            "{name}: outcome"
        );
        let mut weak = envelope;
        weak.artifacts.contract = None;
        weak.artifacts.facts = None;
        weak.artifacts.report = None;
        assert!(
            weak.validate(EvidenceProfile::EngineBacked).is_err(),
            "{name}: stronger capability"
        );
    }
}
