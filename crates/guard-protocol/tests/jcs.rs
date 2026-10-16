use guard_protocol::canonicalize;

#[test]
fn rfc8785_appendix_b_numbers_match_exact_ecmascript_bytes() {
    let data: serde_json::Value =
        serde_json::from_str(include_str!("../fixtures/rfc8785-appendix-b.json")).unwrap();
    let cases = data["vectors"].as_array().unwrap();
    assert_eq!(cases.len(), 24);
    for row in cases {
        let bits = u64::from_str_radix(row["ieee754"].as_str().unwrap(), 16).unwrap();
        let expected = row["canonical"].as_str().unwrap();

        let raw = serde_json::to_vec(&f64::from_bits(bits)).unwrap();
        assert_eq!(
            canonicalize(&raw).unwrap(),
            expected.as_bytes(),
            "IEEE754={bits:016x}"
        );
    }
}

#[test]
fn numeric_lexical_forms_normalize_and_invalid_numbers_fail_closed() {
    for (raw, expected) in [
        ("1.0", "1"),
        ("1e0", "1"),
        ("-0", "0"),
        ("1E+30", "1e+30"),
        ("0.000001", "0.000001"),
        ("0.0000001", "1e-7"),
        ("333333333.33333329", "333333333.3333333"),
        ("9007199254740993", "9007199254740992"),
    ] {
        assert_eq!(canonicalize(raw.as_bytes()).unwrap(), expected.as_bytes());
    }
    for raw in [
        "NaN",
        "Infinity",
        "-Infinity",
        "1e309",
        "-1e309",
        "01",
        "+1",
        ".1",
        "1.",
        "1e",
    ] {
        assert!(canonicalize(raw.as_bytes()).is_err(), "{raw}");
    }
    for raw in [
        br#"{"x":1.0,"x":1e0}"#.as_slice(),
        br#"{"outer":{"x":-0,"\u0078":0}}"#,
        br#"{"bad":"\ud800"}"#,
        br#"{"bad":"\udfff"}"#,
    ] {
        assert!(canonicalize(raw).is_err());
    }
}

#[test]
fn integer_counter_limits_cannot_be_bypassed_with_float_or_exponent_notation() {
    let raw = include_str!("../fixtures/valid/verification-plan.json");
    let oversized = raw.replace("30000", "9007199254740992.0");
    assert!(guard_protocol::decode(oversized.as_bytes()).is_err());
}
