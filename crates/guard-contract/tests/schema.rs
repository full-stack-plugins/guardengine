use guard_contract::SchemaValidator;
use serde_json::json;

#[test]
fn real_draft202012_constraints_are_evaluated_not_just_deserialized() {
    let schema = json!({"$schema":"https://json-schema.org/draft/2020-12/schema",
        "type":"object", "$defs":{"positive":{"type":"integer","minimum":1}},
        "properties":{"mode":{"enum":["strict","review"]},"limit":{"$ref":"#/$defs/positive"}},
        "required":["mode"],"allOf":[{"if":{"properties":{"mode":{"const":"strict"}}},"then":{"required":["limit"]}}],
        "unevaluatedProperties":false});
    let validator = SchemaValidator::build(&serde_json::to_vec(&schema).unwrap()).unwrap();
    assert!(
        validator
            .validate(&json!({"mode":"strict","limit":2}))
            .is_empty()
    );
    for value in [
        json!({"mode":"strict"}),
        json!({"mode":"strict","limit":0}),
        json!({"mode":"review","extra":true}),
    ] {
        assert!(!validator.validate(&value).is_empty());
    }
}

#[test]
fn external_references_are_rejected_without_network_or_file_resolution() {
    for reference in [
        "https://127.0.0.1:1/no-network.json",
        "file:///E:/must-not-read.json",
    ] {
        let bytes = serde_json::to_vec(&json!({"$ref":reference})).unwrap();
        assert!(SchemaValidator::build(&bytes).is_err());
    }
    assert!(SchemaValidator::build(br#"{"type":"object","type":"string"}"#).is_err());
}

#[test]
fn schema_data_annotations_are_not_mistaken_for_external_schema_references() {
    let schema = json!({"$schema":"https://json-schema.org/draft/2020-12/schema",
        "const":{"$ref":"literal-data-not-a-fetch"},"examples":[{"$ref":"https://example.invalid/data"}]});
    let validator = SchemaValidator::build(&serde_json::to_vec(&schema).unwrap()).unwrap();
    assert!(
        validator
            .validate(&json!({"$ref":"literal-data-not-a-fetch"}))
            .is_empty()
    );
}

#[test]
fn unknown_required_vocabulary_and_formats_fail_closed() {
    for schema in [
        json!({"$vocabulary":{"https://example.invalid/unsupported":true}}),
        json!({"type":"string","format":"unknown-required-business-format"}),
    ] {
        assert!(SchemaValidator::build(&serde_json::to_vec(&schema).unwrap()).is_err());
    }
}

#[test]
fn protocol_corpus_uses_both_real_schema_and_separate_semantic_validation() {
    let validator = SchemaValidator::protocol().unwrap();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../guard-protocol/fixtures");
    let cases: Vec<serde_json::Value> =
        serde_json::from_slice(&std::fs::read(root.join("cases.json")).unwrap()).unwrap();
    let mut structural = 0;
    let mut semantic = 0;
    let mut valid = 0;
    for case in cases {
        let bytes = std::fs::read(root.join(case["file"].as_str().unwrap())).unwrap();
        let schema_valid = validator
            .validate_bytes(&bytes)
            .is_ok_and(|issues| issues.is_empty());
        let protocol_valid = guard_protocol::decode(&bytes).is_ok();
        if case["valid"] == true {
            assert!(schema_valid && protocol_valid, "{}", case["file"]);
            valid += 1;
        } else {
            assert!(!protocol_valid, "{}", case["file"]);
            if schema_valid {
                semantic += 1;
            } else {
                structural += 1;
            }
        }
    }
    assert!(valid >= 22 && structural >= 20 && semantic > 0);
    println!(
        "protocol corpus: {valid} valid, {structural} raw/Schema rejected, {semantic} separately semantic rejected"
    );
}

#[test]
fn embedded_schema_resources_enforce_their_required_vocabulary() {
    let schema = json!({"$defs":{"embedded":{"$id":"https://example.invalid/embedded",
        "$schema":"https://json-schema.org/draft/2020-12/schema",
        "$vocabulary":{"https://example.invalid/unsupported":true}}},"$ref":"#/$defs/embedded"});
    assert!(SchemaValidator::build(&serde_json::to_vec(&schema).unwrap()).is_err());
    let data = json!({"const":{"$vocabulary":{"https://example.invalid/unsupported":true}}});
    assert!(SchemaValidator::build(&serde_json::to_vec(&data).unwrap()).is_ok());
}
