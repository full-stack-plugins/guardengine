use guard_policy::{OpaConfig, PolicyPackage};
use guard_protocol::{Digest, ExecutionBudget, Identifier};
use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};
use std::collections::BTreeMap;
fn hash(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}
fn setup(caps: &[u8]) -> (tempfile::TempDir, Value) {
    let temp = tempfile::tempdir().unwrap();
    let source = b"package fixture\nimport rego.v1\ndecision := true\n";
    std::fs::write(temp.path().join("gate.rego"), source).unwrap();
    std::fs::write(temp.path().join("capabilities.json"), caps).unwrap();
    let manifest = json!({"schemaVersion":1,"packageId":"test.policy","version":"1","owner":"flowguard","entrypoint":"data.fixture.decision","files":[{"path":"gate.rego","digest":hash(source)}],"capabilities":{"path":"capabilities.json","digest":hash(caps)}});
    (temp, manifest)
}
fn load(temp: &tempfile::TempDir, m: &Value) -> Result<PolicyPackage, String> {
    PolicyPackage::load(temp.path(), &serde_json::to_vec(m).unwrap())
}
#[test]
fn closed_manifest_identity_is_jcs_and_freezes_raw_bytes() {
    let (temp, m) = setup(br#"{"builtins":[],"allow_net":[]}"#);
    let package = load(&temp, &m).unwrap();
    assert_eq!(
        package.digest(),
        &guard_protocol::digest_json(&serde_json::to_vec(&m).unwrap()).unwrap()
    );
    std::fs::write(temp.path().join("gate.rego"), b"changed").unwrap();
    assert!(load(&temp, &m).is_err());
    assert_eq!(
        package.digest(),
        &guard_protocol::digest_json(&serde_json::to_vec(&m).unwrap()).unwrap()
    );
}
#[test]
fn unsafe_or_remote_capabilities_are_rejected() {
    for caps in [
        json!({"builtins":[{"name":"http.send"}],"allow_net":[]}),
        json!({"builtins":[{"name":"time.now_ns"}],"allow_net":[]}),
        json!({"builtins":[],"allow_net":["example.com"]}),
        json!({"builtins":[]}),
        json!({"builtins":[],"allow_net":[],"unknown":true}),
    ] {
        let (temp, m) = setup(&serde_json::to_vec(&caps).unwrap());
        assert!(load(&temp, &m).is_err(), "{caps}");
    }
}
#[test]
fn paths_queries_unknown_fields_aliases_and_core_owner_are_rejected() {
    let (temp, m) = setup(br#"{"builtins":[],"allow_net":[]}"#);
    for path in [
        "../gate.rego",
        "/gate.rego",
        "gate.rego/",
        "a//gate.rego",
        "NUL.rego",
        "a\\gate.rego",
    ] {
        let mut invalid = m.clone();
        invalid["files"][0]["path"] = json!(path);
        assert!(load(&temp, &invalid).is_err(), "{path}");
    }
    for query in [
        "data.fixture.decision; true",
        "data.fixture[\"decision\"]",
        "input.decision",
        "data.",
    ] {
        let mut invalid = m.clone();
        invalid["entrypoint"] = json!(query);
        assert!(load(&temp, &invalid).is_err());
    }
    let mut invalid = m.clone();
    invalid["trusted"] = json!(true);
    assert!(load(&temp, &invalid).is_err());
    invalid = m.clone();
    invalid["owner"] = json!("core");
    assert!(load(&temp, &invalid).is_err());
    invalid = m;
    invalid["files"]
        .as_array_mut()
        .unwrap()
        .push(json!({"path":"GATE.rego","digest":hash(b"unused")}));
    assert!(load(&temp, &invalid).is_err());
}
#[test]
fn explicit_environment_must_not_override_frozen_argv_or_alias_windows_names() {
    let (temp, m) = setup(br#"{"builtins":[],"allow_net":[]}"#);
    let package = load(&temp, &m).unwrap();
    let mut config = OpaConfig {
        executable: std::env::current_exe().unwrap(),
        executable_digest: Digest::new(hash(b"fixture")).unwrap(),
        version: Identifier::new("1.21.2").unwrap(),
        environment: BTreeMap::new(),
        budget: ExecutionBudget {
            timeout_ms: 2000,
            max_output_bytes: 10000,
            max_concurrency: 1,
        },
    };
    assert!(config.configuration_digest(&package).is_ok());
    config
        .environment
        .insert("OPA_EVAL_QUERY".into(), "true".into());
    assert!(config.configuration_digest(&package).is_err());
    config.environment.clear();
    config.environment.insert("Path".into(), "a".into());
    config.environment.insert("PATH".into(), "b".into());
    assert!(config.configuration_digest(&package).is_err());
}
