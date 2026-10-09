use guardengine::{
    API_VERSION, AnalyzerIdentity, Completeness, ContractMetadata, ContractSpec, Decision,
    Enforcement, GuardAssertion, GuardContract, GuardFact, GuardFacts, GuardRule, GuardSubject,
    RuleStatus, evaluate, verify_report,
};

fn contract(enforcement: Enforcement) -> GuardContract {
    GuardContract {
        api_version: API_VERSION.into(),
        kind: "GuardContract".into(),
        metadata: ContractMetadata {
            id: "agent-job-contract".into(),
            revision: "1".into(),
        },
        spec: ContractSpec {
            rules: vec![GuardRule {
                id: "ARCH-001".into(),
                description: "Do not depend on SaaS".into(),
                enforcement,
                assertion: GuardAssertion::ForbidRelation {
                    subject: "agent-job".into(),
                    predicate: "depends_on".into(),
                    object: "agent-saas".into(),
                },
            }],
        },
    }
}
fn facts(prohibited: bool, complete: bool) -> GuardFacts {
    let facts = if prohibited {
        vec![GuardFact {
            subject: "agent-job".into(),
            predicate: "depends_on".into(),
            object: "agent-saas".into(),
            source: "agent-job/Cargo.toml".into(),
        }]
    } else {
        vec![GuardFact {
            subject: "agent-job".into(),
            predicate: "depends_on".into(),
            object: "agent-contracts".into(),
            source: "agent-job/Cargo.toml".into(),
        }]
    };
    GuardFacts {
        api_version: API_VERSION.into(),
        kind: "GuardFacts".into(),
        analyzer: AnalyzerIdentity {
            id: "archguard.cargo".into(),
            version: "0.1.0".into(),
        },
        subject: GuardSubject {
            id: "example".into(),
            snapshot_digest: "sha256:fixture".into(),
        },
        completeness: if complete {
            Completeness::Complete
        } else {
            Completeness::Partial
        },
        facts,
        diagnostics: if complete {
            vec![]
        } else {
            vec!["source file skipped".into()]
        },
    }
}

#[test]
fn allowed_relation_is_pass() {
    let report = evaluate(&contract(Enforcement::Enforce), &facts(false, true)).unwrap();
    assert_eq!(report.decision, Decision::Allow);
    assert_eq!(report.evaluations[0].status, RuleStatus::Pass);
    assert!(!report.signed);
}

#[test]
fn forbidden_relation_blocks() {
    let report = evaluate(&contract(Enforcement::Enforce), &facts(true, true)).unwrap();
    assert_eq!(report.decision, Decision::Block);
    assert_eq!(report.evaluations[0].status, RuleStatus::Fail);
    assert_eq!(report.evaluations[0].matched_facts.len(), 1);
}

#[test]
fn review_policy_needs_approval() {
    let report = evaluate(&contract(Enforcement::Review), &facts(true, true)).unwrap();
    assert_eq!(report.decision, Decision::RequireApproval);
    assert_eq!(report.evaluations[0].status, RuleStatus::ReviewRequired);
}

#[test]
fn advisory_does_not_block_but_keeps_matches() {
    let report = evaluate(&contract(Enforcement::Advise), &facts(true, true)).unwrap();
    assert_eq!(report.decision, Decision::Allow);
    assert_eq!(report.evaluations[0].matched_facts.len(), 1);
    assert!(report.evaluations[0].detail.contains("advisory"));
}

#[test]
fn incomplete_facts_fail_closed() {
    let report = evaluate(&contract(Enforcement::Enforce), &facts(false, false)).unwrap();
    assert_eq!(report.decision, Decision::Block);
    assert_eq!(report.evaluations[0].status, RuleStatus::Indeterminate);
}

#[test]
fn report_is_reproducible_and_detects_tampering() {
    let c = contract(Enforcement::Enforce);
    let f = facts(true, true);
    let first = evaluate(&c, &f).unwrap();
    let second = evaluate(&c, &f).unwrap();
    assert_eq!(first, second);
    assert!(verify_report(&first, &c, &f).unwrap());
    let mut altered = first.clone();
    altered.evaluations[0].detail = "PASS".into();
    assert!(!verify_report(&altered, &c, &f).unwrap());
}

#[test]
fn rejects_duplicate_rules_and_unknown_versions() {
    let mut c = contract(Enforcement::Enforce);
    c.spec.rules.push(c.spec.rules[0].clone());
    assert!(c.validate().is_err());
    c.spec.rules.pop();
    c.api_version = "guard.partme.ai/v99".into();
    assert!(c.validate().is_err());
}

#[test]
fn yaml_rejects_misspelled_critical_properties() {
    let text = r#"
apiVersion: guard.partme.ai/v1alpha1
kind: GuardContract
metadata: { id: example, revision: "1" }
spec:
  rules:
    - id: ARCH-001
      enforcement: enforce
      assertion: { type: forbid_relation, subject: a, predicat: depends_on, object: b }
"#;
    assert!(guardengine::load_contract_yaml(text.as_bytes()).is_err());
}
