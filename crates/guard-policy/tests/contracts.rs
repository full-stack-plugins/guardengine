#[test]
fn undefined_or_malformed_policy_output_never_defaults_to_allow() {
    use guard_policy::PolicyDecision;
    assert!(PolicyDecision::from_opa_json(br#"{}"#, "data.fixture.decision").is_err());
    assert!(PolicyDecision::from_opa_json(br#"{"result":[{"expressions":[{"value":{"schemaVersion":1,"decision":"ALLOW","reasons":[],"requiredActions":[],"missingObligations":[],"activeViolations":[],"trusted":true},"text":"data.fixture.decision","location":{"row":1,"col":1}}]}]}"#, "data.fixture.decision").is_err());
}
#[test]
fn missing_opa_envelope_fields_return_errors_without_panicking() {
    for raw in [
        r#"{"result":[{}]}"#,
        r#"{"result":[{"bindings":{}}]}"#,
        r#"{"result":[{"expressions":[{"text":"data.fixture.decision","value":{},"other":1}]}]}"#,
        r#"{"result":[{"expressions":[{"text":"data.fixture.decision","location":{"row":1,"col":1},"other":1}]}]}"#,
        r#"{"result":[{"expressions":[{"a":1,"b":2,"c":3}]}]}"#,
    ] {
        let result = std::panic::catch_unwind(|| {
            guard_policy::PolicyDecision::from_opa_json(raw.as_bytes(), "data.fixture.decision")
        });
        assert!(result.is_ok(), "untrusted missing fields cannot panic");
        assert!(result.unwrap().is_err());
    }
}
