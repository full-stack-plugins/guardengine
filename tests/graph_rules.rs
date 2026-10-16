use guardengine::{Decision, RuleStatus, evaluate, load_contract_yaml, load_facts_json};
use serde_json::{Value, json};

fn facts() -> guardengine::GuardFacts {
    load_facts_json(&serde_json::to_vec(&json!({
        "apiVersion":"guard.partme.ai/v1alpha2", "kind":"GuardFacts",
        "analyzer":{"id":"fixture.graph","version":"1"},
        "subject":{"id":"example","snapshotDigest":"sha256:fixture"},
        "completeness":"complete", "coverage":{"entities":["app","domain","infra"],"predicates":["depends_on"],"scope":"fixture declared graph"},
        "facts":[
            {"subject":"app","predicate":"depends_on","object":"domain","source":"app/pom.xml:12"},
            {"subject":"domain","predicate":"depends_on","object":"infra","source":"domain/pom.xml:20"}
        ], "diagnostics":[]
    })).unwrap()).unwrap()
}
fn contract(assertion: Value) -> guardengine::GuardContract {
    load_contract_yaml(
        &serde_json::to_vec(&json!({
            "apiVersion":"guard.partme.ai/v1alpha2","kind":"GuardContract",
            "metadata":{"id":"layered","revision":"1"},
            "spec":{"rules":[{"id":"ARCH-1","enforcement":"enforce","assertion":assertion}]}
        }))
        .unwrap(),
    )
    .unwrap()
}

#[test]
fn forbidden_path_has_actual_chain_and_is_deterministic() {
    let c = contract(
        json!({"type":"forbid_path","subject":"app","predicate":"depends_on","object":"infra"}),
    );
    let f = facts();
    let report = evaluate(&c, &f).unwrap();
    assert_eq!(report.decision, Decision::Block);
    assert_eq!(report.evaluations[0].matched_facts.len(), 2);
    let mut reversed = f.clone();
    reversed.facts.reverse();
    assert_eq!(report, evaluate(&c, &reversed).unwrap());
}
#[test]
fn allowed_dependencies_and_required_relation_detect_violations() {
    for assertion in [
        json!({"type":"allowed_dependencies","subject":"app","predicate":"depends_on","objects":["infra"]}),
        json!({"type":"require_relation","subject":"app","predicate":"depends_on","object":"infra"}),
    ] {
        assert_eq!(
            evaluate(&contract(assertion), &facts()).unwrap().decision,
            Decision::Block
        );
    }
}
#[test]
fn cycle_evidence_includes_each_edge() {
    let c = contract(json!({"type":"acyclic","predicate":"depends_on"}));
    let mut f = facts();
    assert_eq!(evaluate(&c, &f).unwrap().decision, Decision::Allow);
    f.facts.push(guardengine::GuardFact {
        subject: "infra".into(),
        predicate: "depends_on".into(),
        object: "app".into(),
        source: "infra/pom.xml:8".into(),
    });
    let report = evaluate(&c, &f).unwrap();
    assert_eq!(report.decision, Decision::Block);
    assert_eq!(report.evaluations[0].matched_facts.len(), 3);
}
#[test]
fn missing_entity_or_predicate_is_not_a_pass() {
    for assertion in [
        json!({"type":"forbid_relation","subject":"typo","predicate":"depends_on","object":"infra"}),
        json!({"type":"acyclic","predicate":"calls"}),
    ] {
        let result = evaluate(&contract(assertion), &facts()).unwrap();
        assert_eq!(result.decision, Decision::Block);
        assert_eq!(result.evaluations[0].status, RuleStatus::Indeterminate);
    }
}

#[test]
fn legacy_facts_cannot_satisfy_v2_and_legacy_contracts_keep_exact_semantics() {
    let mut f = facts();
    f.api_version = guardengine::API_VERSION.into();
    f.coverage = None;
    let mut c = contract(
        json!({"type":"forbid_relation","subject":"absent","predicate":"depends_on","object":"infra"}),
    );
    assert_eq!(evaluate(&c, &f).unwrap().decision, Decision::Block);
    c.api_version = guardengine::API_VERSION.into();
    assert_eq!(evaluate(&c, &f).unwrap().decision, Decision::Allow);
}
#[test]
fn strict_coverage_rejects_missing_inventory_and_outside_edges() {
    let mut f = facts();
    f.coverage.as_mut().unwrap().entities.clear();
    assert!(f.validate().is_err());
    let mut f = facts();
    f.facts[0].object = "outside".into();
    assert!(f.validate().is_err());
    let mut f = facts();
    f.coverage = None;
    assert!(f.validate().is_err());
}
#[test]
fn coverage_order_does_not_change_evidence() {
    let c = contract(json!({"type":"acyclic","predicate":"depends_on"}));
    let f = facts();
    let mut changed = f.clone();
    changed.coverage.as_mut().unwrap().entities.reverse();
    assert_eq!(evaluate(&c, &f).unwrap(), evaluate(&c, &changed).unwrap());
}
#[test]
fn graph_review_advice_and_unknown_coverage_have_distinct_decisions() {
    let mut c = contract(
        json!({"type":"forbid_path","subject":"app","predicate":"depends_on","object":"infra"}),
    );
    c.spec.rules[0].enforcement = guardengine::Enforcement::Review;
    assert_eq!(
        evaluate(&c, &facts()).unwrap().decision,
        Decision::RequireApproval
    );
    c.spec.rules[0].enforcement = guardengine::Enforcement::Advise;
    let result = evaluate(&c, &facts()).unwrap();
    assert_eq!(result.decision, Decision::Allow);
    assert_eq!(result.evaluations[0].matched_facts.len(), 2);
    let mut f = facts();
    f.completeness = guardengine::Completeness::Partial;
    f.diagnostics.push("unresolved parent".into());
    assert_eq!(evaluate(&c, &f).unwrap().decision, Decision::Block);
}
#[test]
fn self_cycles_and_disconnected_cycles_have_actual_closed_paths() {
    let c = contract(json!({"type":"acyclic","predicate":"depends_on"}));
    let mut f = facts();
    f.facts.push(guardengine::GuardFact {
        subject: "infra".into(),
        predicate: "depends_on".into(),
        object: "infra".into(),
        source: "infra/pom.xml:1".into(),
    });
    let report = evaluate(&c, &f).unwrap();
    let witness = &report.evaluations[0].matched_facts;
    assert_eq!(witness.len(), 1);
    assert_eq!(witness[0].subject, witness[0].object);
    f.facts.pop();
    f.facts.push(guardengine::GuardFact {
        subject: "infra".into(),
        predicate: "depends_on".into(),
        object: "domain".into(),
        source: "infra/pom.xml:1".into(),
    });
    let report = evaluate(&c, &f).unwrap();
    let witness = &report.evaluations[0].matched_facts;
    assert_eq!(witness.len(), 2);
    assert_eq!(
        witness.first().unwrap().subject,
        witness.last().unwrap().object
    );
}
