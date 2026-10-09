use guardengine::integration::{FactBudget, MAX_ARTIFACT_BYTES};
#[test]
fn borrowed_fields_preserve_native_fact() {
    let mut b = FactBudget::new();
    b.push_relation("subject", "predicate", "object", "source")
        .unwrap();
    let facts = b.finish().unwrap();
    assert_eq!(facts.len(), 1);
    assert_eq!(facts[0].object, "object");
}
#[test]
fn oversized_borrowed_field_poison_prevents_partial_success() {
    let mut b = FactBudget::new();
    b.push_relation("s", "p", "o", "source").unwrap();
    let large = "x".repeat(MAX_ARTIFACT_BYTES);
    assert!(b.push_relation(&large, "p", "o", "source").is_err());
    assert!(b.finish().is_err());
}
#[test]
fn cumulative_amplification_rejected_before_next_clone() {
    let field = "x".repeat(64 * 1024);
    let mut b = FactBudget::new();
    let mut accepted = 0;
    for _ in 0..96 {
        if b.push_relation(&field, &field, &field, "native").is_err() {
            break;
        }
        accepted += 1;
    }
    assert!(accepted > 0 && accepted < 86);
    assert!(b.finish().is_err());
}
#[test]
fn escaped_serialized_size_and_count_are_bounded() {
    let mut b = FactBudget::new();
    let escaped = "\u{0001}".repeat(MAX_ARTIFACT_BYTES / 5);
    assert!(b.push_relation(&escaped, "p", "o", "s").is_err());
    let mut b = FactBudget::new();
    for _ in 0..4096 {
        b.push_relation("s", "p", "o", "source").unwrap();
    }
    assert!(b.push_relation("s", "p", "o", "source").is_err());
    assert!(b.finish().is_err());
}
