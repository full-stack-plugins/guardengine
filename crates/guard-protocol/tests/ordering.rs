use guard_protocol::RunSequence;

#[test]
fn sequence_order_uses_u64_value_instead_of_wire_lexicography() {
    let n = |s: &str| RunSequence::new(s).unwrap();
    assert!(n("9") < n("10"));
    assert!(n("2") < n("10"));
    assert!(n("999") < n("1000"));
    assert!(n("18446744073709551614") < n("18446744073709551615"));
    assert!(n("0") < n("18446744073709551615"));
    assert!(RunSequence::new("18446744073709551616").is_err());
    assert!(RunSequence::new("01").is_err());
    assert_eq!(serde_json::to_string(&n("10")).unwrap(), "\"10\"");
}
#[test]
fn timestamp_order_compares_instants_not_wire_punctuation() {
    let t = |s: &str| guard_protocol::UtcTimestamp::new(s).unwrap();
    let whole = t("2026-10-17T00:00:00Z");
    let fractional = t("2026-10-17T00:00:00.1Z");
    assert_eq!(whole.compare_instant(&fractional), std::cmp::Ordering::Less);
    assert_eq!(
        fractional.compare_instant(&t("2026-10-17T00:00:00.10Z")),
        std::cmp::Ordering::Equal
    );
}

#[test]
fn timestamp_wire_identity_is_distinct_from_instant_equality() {
    let a = guard_protocol::UtcTimestamp::new("2026-10-17T00:00:00.1Z").unwrap();
    let b = guard_protocol::UtcTimestamp::new("2026-10-17T00:00:00.10Z").unwrap();
    assert_ne!(a, b);
    assert_eq!(a.compare_instant(&b), std::cmp::Ordering::Equal);
    assert_eq!(
        serde_json::to_string(&a).unwrap(),
        "\"2026-10-17T00:00:00.1Z\""
    );
}
