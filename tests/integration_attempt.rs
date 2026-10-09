use guardengine::integration::{
    Artifacts, AttemptOutput, Diagnostic, EvidenceProfile, GuardRunEnvelope, InvocationDraft,
    RunStatus, prepare_attempt,
};
fn draft() -> InvocationDraft {
    let e: GuardRunEnvelope = serde_json::from_slice(include_bytes!(
        "fixtures/integration-envelope/valid-native.json"
    ))
    .unwrap();
    InvocationDraft {
        run_id: e.run_id,
        producer: Some(e.producer),
        binding: Some(e.binding),
        coverage: Some(e.coverage),
        profile: Some(EvidenceProfile::NativeOnly),
        started_at: e.started_at,
    }
}
#[test]
fn unresolved_candidate_has_no_envelope() {
    let mut d = draft();
    d.binding = None;
    let err = prepare_attempt(d)
        .err()
        .expect("unresolved binding must fail");
    let json = serde_json::to_value(err).unwrap();
    assert_eq!(json["code"], "binding.unresolved");
    assert!(json.get("binding").is_none());
    assert!(json.get("decision").is_none());
    for field in ["producer", "coverage", "profile"] {
        let mut d = draft();
        match field {
            "producer" => d.producer = None,
            "coverage" => d.coverage = None,
            _ => d.profile = None,
        };
        assert!(prepare_attempt(d).is_err());
    }
    let mut d = draft();
    d.binding.as_mut().unwrap().candidate_oid = "main".into();
    assert!(prepare_attempt(d).is_err());
    assert!(prepare_attempt(draft()).is_ok());
}
#[test]
fn bound_failure_keeps_null_verdict_and_original_binding() {
    let expected = draft().binding.unwrap();
    for status in [RunStatus::Error, RunStatus::Cancelled] {
        let attempt = prepare_attempt(draft()).unwrap();
        let output = AttemptOutput {
            coverage: draft().coverage.unwrap(),
            run_status: status.clone(),
            decision: None,
            artifacts: Artifacts {
                contract: None,
                facts: None,
                report: None,
                domain: vec![],
            },
            approval_refs: vec![],
            diagnostics: vec![Diagnostic {
                code: "runner.failed".into(),
                message: "failed".into(),
                retryable: true,
                source: None,
            }],
            finished_at: "2026-10-09T10:00:01Z".into(),
            expires_at: None,
        };
        let e = attempt.finish(output).unwrap();
        assert_eq!(e.binding, expected);
        assert_eq!(e.decision, None);
        assert_eq!(e.run_status, status);
    }
}

#[test]
fn required_scope_freezes_but_observed_coverage_is_an_outcome() {
    use guardengine::{Decision, integration::CoverageStatus};
    let original: GuardRunEnvelope = serde_json::from_slice(include_bytes!(
        "fixtures/integration-envelope/valid-native.json"
    ))
    .unwrap();
    let mut initial = draft();
    let coverage = initial.coverage.as_mut().unwrap();
    coverage.status = CoverageStatus::Partial;
    coverage.observed_scopes.clear();
    coverage.missing_scopes = coverage.required_scopes.clone();
    let output = AttemptOutput {
        coverage: original.coverage.clone(),
        run_status: RunStatus::Completed,
        decision: Some(Decision::Allow),
        artifacts: original.artifacts.clone(),
        approval_refs: vec![],
        diagnostics: vec![],
        finished_at: original.finished_at.clone(),
        expires_at: None,
    };
    let completed = prepare_attempt(initial).unwrap().finish(output).unwrap();
    assert_eq!(completed.coverage.status, CoverageStatus::Complete);
    let mut weaker = original.coverage.clone();
    weaker.required_scopes = vec!["other".into()];
    weaker.observed_scopes = vec!["other".into()];
    let output = AttemptOutput {
        coverage: weaker,
        run_status: RunStatus::Completed,
        decision: Some(Decision::Allow),
        artifacts: original.artifacts,
        approval_refs: vec![],
        diagnostics: vec![],
        finished_at: original.finished_at,
        expires_at: None,
    };
    assert!(prepare_attempt(draft()).unwrap().finish(output).is_err());
}
