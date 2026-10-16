use guard_protocol::{TrustLevel, canonicalize, decode, project_legacy};

#[test]
fn duplicate_keys_unknown_versions_and_nonfinite_numbers_are_rejected() {
    for input in [
        r#"{"a":1,"a":2}"#,
        r#"{"a":{"x":1,"x":2}}"#,
        r#"{"number":1e999}"#,
        r#"{"number":NaN}"#,
    ] {
        assert!(canonicalize(input.as_bytes()).is_err(), "{input}");
    }
    assert!(decode(br#"{"schemaVersion":"2.0.0","documentType":"GuardResult"}"#).is_err());
}

#[test]
fn canonical_bytes_sort_utf16_and_preserve_path_identity() {
    assert_eq!(
        canonicalize("{\"\\ue000\":1,\"😀\":2,\"a\":0}".as_bytes()).unwrap(),
        "{\"a\":0,\"😀\":2,\"\\ue000\":1}"
            .replace("\\ue000", "\u{e000}")
            .as_bytes()
    );
    assert_eq!(
        canonicalize(br#"{ "b": [true,null], "a":"\u0061" }"#).unwrap(),
        br#"{"a":"a","b":[true,null]}"#
    );
}

#[test]
fn legacy_bytes_survive_without_authority_promotion() {
    let source = br#"{ "schema_version":"0.2.0", "delivery_decision":"allow", "trusted":true }"#;
    let observation = project_legacy(source, "codeguard/v1").unwrap();
    assert_eq!(observation.original_bytes(), source);
    assert_eq!(observation.trust_level(), TrustLevel::Unverified);
    assert_eq!(observation.decision(), "NOT_EVALUATED");
    assert!(project_legacy(br#"{"schema_version":"9.0.0"}"#, "codeguard/v1").is_err());
}
