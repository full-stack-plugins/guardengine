use guard_protocol::{Digest, GitPath, Identifier, TargetSelection, decode, encode};
use serde_json::Value;

fn contract() -> Value {
    serde_json::from_slice(include_bytes!("../fixtures/valid/contract-snapshot.json")).unwrap()
}

#[test]
fn approved_contract_selects_scope_without_freezing_every_candidate_content() {
    let mut value = contract();
    value["obligations"][0]["targets"][0]
        .as_object_mut()
        .unwrap()
        .remove("inputDigest");
    let parsed = decode(&serde_json::to_vec(&value).unwrap())
        .expect("contract selections do not need candidate inputDigest");
    let canonical: Value = serde_json::from_slice(&encode(parsed.document()).unwrap()).unwrap();
    assert!(
        canonical["obligations"][0]["targets"][0]
            .get("inputDigest")
            .is_none()
    );
}

#[test]
fn contract_optional_protected_digest_is_distinct_from_plan_input_digest() {
    let mut value = contract();
    let target = value["obligations"][0]["targets"][0]
        .as_object_mut()
        .unwrap();
    target.remove("inputDigest");
    target.insert(
        "protectedDigest".into(),
        Value::String(format!("sha256:{}", "b".repeat(64))),
    );
    assert!(decode(&serde_json::to_vec(&value).unwrap()).is_ok());
    let mut plan: Value =
        serde_json::from_slice(include_bytes!("../fixtures/valid/verification-plan.json")).unwrap();
    plan["obligations"][0]["targets"][0]
        .as_object_mut()
        .unwrap()
        .remove("inputDigest");
    assert!(
        decode(&serde_json::to_vec(&plan).unwrap()).is_err(),
        "plan cannot leave actual candidate input unbound"
    );
}

#[test]
fn rebinding_unprotected_selection_preserves_scope_and_protected_selection_rejects_drift() {
    let first = Digest::new(format!("sha256:{}", "a".repeat(64))).unwrap();
    let second = Digest::new(format!("sha256:{}", "b".repeat(64))).unwrap();
    let mut selection = TargetSelection {
        repository_id: Identifier::new("repo").unwrap(),
        path: GitPath::from_bytes(b"src/Example.java").unwrap(),
        selector: Some(Identifier::new("Order#submit").unwrap()),
        protected_digest: None,
    };
    let before = serde_json::to_vec(&selection).unwrap();
    for digest in [&first, &second] {
        let bound = selection.bind(digest.clone()).unwrap();
        assert_eq!(bound.repository_id, selection.repository_id);
        assert_eq!(bound.path, selection.path);
        assert_eq!(bound.selector, selection.selector);
        assert_eq!(&bound.input_digest, digest);
    }
    assert_eq!(
        serde_json::to_vec(&selection).unwrap(),
        before,
        "candidate edits do not mutate approved selection"
    );
    selection.protected_digest = Some(first.clone());
    assert!(selection.bind(first).is_ok());
    assert!(selection.bind(second).is_err());
}

#[test]
fn duplicate_scope_and_cross_stage_fields_are_rejected() {
    let mut value = contract();
    let mut duplicate = value["obligations"][0]["targets"][0].clone();
    duplicate["protectedDigest"] = Value::String(format!("sha256:{}", "a".repeat(64)));
    value["obligations"][0]["targets"]
        .as_array_mut()
        .unwrap()
        .push(duplicate);
    assert!(decode(&serde_json::to_vec(&value).unwrap()).is_err());
    let mut value = contract();
    value["obligations"][0]["targets"][0]["inputDigest"] =
        Value::String(format!("sha256:{}", "a".repeat(64)));
    assert!(
        decode(&serde_json::to_vec(&value).unwrap()).is_err(),
        "candidate-only field is forbidden in approved target selection"
    );
}

#[test]
fn schema_and_rust_keep_contract_selections_separate_from_bound_plan_targets() {
    let schema: Value =
        serde_json::from_slice(include_bytes!("../schemas/engineering.schema.json")).unwrap();
    assert_eq!(
        schema["$defs"]["ContractSnapshot"]["properties"]["obligations"]["items"]["$ref"],
        "#/$defs/ContractObligation"
    );
    assert_eq!(
        schema["$defs"]["ContractObligation"]["properties"]["targets"]["items"]["$ref"],
        "#/$defs/TargetSelection"
    );
    assert_eq!(
        schema["$defs"]["VerificationPlan"]["properties"]["obligations"]["items"]["$ref"],
        "#/$defs/Obligation"
    );
    assert_eq!(
        schema["$defs"]["Obligation"]["properties"]["targets"]["items"]["$ref"],
        "#/$defs/ExactTarget"
    );
}
