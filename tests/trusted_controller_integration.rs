#[path = "support/trusted_controller.rs"]
mod fixture;
use fixture::*;
use guardengine::integration::eligibility::*;
#[test]
fn protected_controller_qualifies_actual_synthetic_merge_without_domain_side_effects() {
    let repo = GitFixture::new();
    let before = repo.snapshot();
    let controller = Controller::freeze(&repo);
    let policy_before = controller.policy().content_digest();
    let output = controller.run_owned_job(&repo);
    assert_eq!(
        controller.evaluate(&output).code,
        EligibilityCode::InvalidApproval
    );
    controller.approve();
    let result = controller.evaluate(&output);
    assert_eq!(result.code, EligibilityCode::Eligible);
    assert_eq!(
        result.technical_decision,
        Some(guardengine::Decision::RequireApproval)
    );
    assert!(result.audit.actor_digest.is_some());
    assert_eq!(result.audit.approver_digests.len(), 1);
    assert_eq!(controller.policy().content_digest(), policy_before);
    assert_eq!(repo.snapshot(), before);
    println!(
        "GE-TRUST {ADAPTER} candidate={} parents={},{} git={} before_after={} audit={}",
        repo.candidate,
        repo.base,
        repo.feature,
        String::from_utf8(repo.git(&["--version"])).unwrap().trim(),
        serde_json::to_string(&before).unwrap(),
        serde_json::to_string(&result.audit).unwrap()
    );
}
#[test]
fn consistent_forgery_drift_and_current_authority_fail_closed() {
    let repo = GitFixture::new();
    let tree = String::from_utf8(repo.git(&["rev-parse", &format!("{}^{{tree}}", repo.candidate)]))
        .unwrap()
        .trim()
        .to_owned();
    let other_candidate = String::from_utf8(repo.git(&[
        "commit-tree",
        &tree,
        "-p",
        &repo.base,
        "-p",
        &repo.feature,
        "-m",
        "different synthetic candidate",
    ]))
    .unwrap()
    .trim()
    .to_owned();
    assert_ne!(other_candidate, repo.candidate);
    let feature_tree =
        String::from_utf8(repo.git(&["rev-parse", &format!("{}^{{tree}}", repo.feature)]))
            .unwrap()
            .trim()
            .to_owned();
    let other_source = digest(
        &serde_json::to_vec(&(
            feature_tree,
            repo.git(&["show", &format!("{}:review.txt", repo.feature)]),
        ))
        .unwrap(),
    );
    let before = repo.snapshot();
    let c = Controller::freeze(&repo);
    let out = c.run_owned_job(&repo);
    c.approve();
    assert!(c.evaluate(&out).eligible);
    let mut forged = out.clone();
    forged.envelope.run_id = "unissued-copy".into();
    guardengine::integration::verify_engine_artifacts(
        &forged.envelope,
        &forged.contract,
        &forged.facts,
        &forged.report,
    )
    .unwrap();
    assert_eq!(c.evaluate(&forged).code, EligibilityCode::UntrustedProducer);
    let mut forged = out.clone();
    forged.envelope.producer.guard = "claimed-producer".into();
    assert!(!c.evaluate(&forged).eligible);
    for n in 0..5 {
        let mut p = c.policy().clone();
        match n {
            0 => p.binding.candidate_oid = other_candidate.clone(),
            1 => p.binding.base_oid = repo.feature.clone(),
            2 => p.binding.merge_group_id = Some("other-group".into()),
            3 => p.binding.source_snapshot_digest = other_source.clone(),
            _ => p.contract_digest = digest(b"other protected policy"),
        };
        assert_eq!(
            guardengine::integration::eligibility::evaluate_eligibility(
                &out.envelope,
                out.artifacts(),
                &p,
                &c,
                NOW,
                None
            )
            .code,
            EligibilityCode::BindingChanged
        );
    }
    struct ApprovalUnavailable<'a>(&'a Controller);
    impl AuthorityProvider for ApprovalUnavailable<'_> {
        fn verify_producer(
            &self,
            e: &guardengine::integration::GuardRunEnvelope,
            d: &str,
        ) -> Result<ProducerRecord, AuthorityError> {
            self.0.verify_producer(e, d)
        }
        fn verify_approval(&self, _: &str) -> Result<ApprovalRecord, AuthorityError> {
            Err(AuthorityError::Unavailable)
        }
    }
    assert_eq!(
        evaluate_eligibility(
            &out.envelope,
            out.artifacts(),
            c.policy(),
            &ApprovalUnavailable(&c),
            NOW,
            None
        )
        .code,
        EligibilityCode::ProviderUnavailable
    );
    assert_eq!(
        evaluate_eligibility(
            &out.envelope,
            out.artifacts(),
            c.policy(),
            &c,
            NOW + 60,
            None
        )
        .code,
        EligibilityCode::UntrustedProducer
    );
    let mut unauthorized = c.policy().clone();
    unauthorized.producer_principals = ["fixture-controller://other/producer".into()].into();
    assert_eq!(
        evaluate_eligibility(&out.envelope, out.artifacts(), &unauthorized, &c, NOW, None).code,
        EligibilityCode::UnauthorizedProducer
    );
    unauthorized = c.policy().clone();
    unauthorized.approval_principals.insert(
        "fixture.review".into(),
        ["fixture-controller://other/reviewer".into()].into(),
    );
    assert_eq!(
        evaluate_eligibility(&out.envelope, out.artifacts(), &unauthorized, &c, NOW, None).code,
        EligibilityCode::InvalidApproval
    );
    c.set_revoked(true, false);
    assert_eq!(c.evaluate(&out).code, EligibilityCode::UntrustedProducer);
    c.set_revoked(false, true);
    assert_eq!(c.evaluate(&out).code, EligibilityCode::InvalidApproval);
    c.set_revoked(false, false);
    c.set_unavailable(true);
    assert_eq!(c.evaluate(&out).code, EligibilityCode::ProviderUnavailable);
    c.set_unavailable(false);
    let calls = c.producer_queries.get();
    let approvals = c.approval_queries.get();
    assert!(c.evaluate(&out).eligible);
    assert_eq!(c.producer_queries.get(), calls + 1);
    assert_eq!(c.approval_queries.get(), approvals + 1);
    assert_eq!(repo.snapshot(), before);
}
