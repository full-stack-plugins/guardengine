use guardengine::Decision;
use guardengine::integration::{eligibility::*, *};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
const NOW: i64 = 1_791_540_002; // 2026-10-09 10:00:02 UTC
fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}
struct Fixture {
    e: GuardRunEnvelope,
    cb: Vec<u8>,
    fb: Vec<u8>,
    rb: Vec<u8>,
    p: EligibilityPolicy,
}
impl Fixture {
    fn new(review: bool, partial: bool) -> Self {
        let cb = serde_json::to_vec(&json!({"apiVersion":guardengine::API_VERSION,"kind":"GuardContract","metadata":{"id":"policy","revision":"1"},"spec":{"rules":[{"id":"deny","description":"test","enforcement":if review {"review"} else {"enforce"},"assertion":{"type":"forbid_relation","subject":"a","predicate":"depends_on","object":"b"}}]}})).unwrap();
        let fb = serde_json::to_vec(&json!({"apiVersion":guardengine::API_VERSION,"kind":"GuardFacts","analyzer":{"id":"native","version":"1"},"subject":{"id":"repo","snapshotDigest":format!("sha256:{}","c".repeat(64))},"completeness":if partial {"partial"} else {"complete"},"facts":if review {json!([{"subject":"a","predicate":"depends_on","object":"b","source":"fixture"}])} else {json!([])},"diagnostics":if partial {vec!["secret diagnostic"]} else {vec![]}})).unwrap();
        let report = guardengine::evaluate(
            &guardengine::load_contract_yaml(&cb).unwrap(),
            &guardengine::load_facts_json(&fb).unwrap(),
        )
        .unwrap();
        let rb = serde_json::to_vec(&report).unwrap();
        let mut v: Value = serde_json::from_slice(include_bytes!(
            "fixtures/integration-envelope/valid-native.json"
        ))
        .unwrap();
        v["producer"]["analyzerId"] = json!("native");
        v["producer"]["analyzerVersion"] = json!("1");
        v["binding"]["sourceSnapshotDigest"] = json!(format!("sha256:{}", "c".repeat(64)));
        v["artifacts"]["domain"] = json!([]);
        v["decision"] = serde_json::to_value(report.decision).unwrap();
        v["startedAt"] = json!("2026-10-09T10:00:00Z");
        v["finishedAt"] = json!("2026-10-09T10:00:01Z");
        v["expiresAt"] = json!("2026-10-09T10:00:10Z");
        if partial {
            v["coverage"]["status"] = json!("partial");
            v["coverage"]["missingScopes"] = v["coverage"]["requiredScopes"].clone();
            v["coverage"]["observedScopes"] = json!([]);
        }
        for (name, bytes) in [("contract", &cb), ("facts", &fb), ("report", &rb)] {
            v["artifacts"][name] = json!({"uri":format!("artifact://local/{name}"),"digest":digest(bytes),"mediaType":"application/json"});
        }
        let e = load_envelope_json(
            &serde_json::to_vec(&v).unwrap(),
            EvidenceProfile::EngineBacked,
        )
        .unwrap();
        let p = EligibilityPolicy {
            binding: e.binding.clone(),
            producer: e.producer.clone(),
            required_scopes: e.coverage.required_scopes.clone(),
            contract_digest: digest(&cb),
            action: "merge".into(),
            producer_principals: ["ci".into()].into(),
            approval_principals: BTreeMap::new(),
        };
        Self { e, cb, fb, rb, p }
    }
    fn provider(&self) -> FixtureAuthority {
        FixtureAuthority {
            producer: Ok(ProducerRecord {
                principal: "ci".into(),
                producer: self.e.producer.clone(),
                envelope_digest: digest(&serde_json::to_vec(&self.e).unwrap()),
                validity: Validity {
                    issued_at: NOW - 2,
                    expires_at: NOW + 8,
                    revoked: false,
                },
            }),
            approvals: BTreeMap::new(),
        }
    }
    fn eval(&self, authority: &FixtureAuthority, now: i64) -> EligibilityResult {
        evaluate_eligibility(
            &self.e,
            ArtifactBytes {
                contract: &self.cb,
                facts: &self.fb,
                report: &self.rb,
            },
            &self.p,
            authority,
            now,
            Some("secret cause"),
        )
    }
    fn approval(&self, purpose: &str) -> ApprovalRecord {
        ApprovalRecord {
            principal: "reviewer".into(),
            purpose: purpose.into(),
            action: self.p.action.clone(),
            binding: self.e.binding.clone(),
            contract_digest: self.p.contract_digest.clone(),
            validity: Validity {
                issued_at: NOW - 2,
                expires_at: NOW + 8,
                revoked: false,
            },
        }
    }
    fn approved() -> (Self, FixtureAuthority) {
        let mut f = Self::new(true, false);
        f.e.approval_refs = vec!["approval-a".into(), "approval-b".into()];
        for purpose in ["security", "release"] {
            f.p.approval_principals
                .insert(purpose.into(), ["reviewer".into()].into());
        }
        let mut a = f.provider();
        a.approvals
            .insert("approval-a".into(), Ok(f.approval("security")));
        a.approvals
            .insert("approval-b".into(), Ok(f.approval("release")));
        (f, a)
    }
}
// Test-only provider: these records do not constitute real authentication.
struct FixtureAuthority {
    producer: Result<ProducerRecord, AuthorityError>,
    approvals: BTreeMap<String, Result<ApprovalRecord, AuthorityError>>,
}
impl AuthorityProvider for FixtureAuthority {
    fn verify_producer(
        &self,
        _: &GuardRunEnvelope,
        _: &str,
    ) -> Result<ProducerRecord, AuthorityError> {
        self.producer.clone()
    }
    fn verify_approval(&self, reference: &str) -> Result<ApprovalRecord, AuthorityError> {
        self.approvals
            .get(reference)
            .cloned()
            .unwrap_or(Err(AuthorityError::Untrusted))
    }
}
#[test]
fn allow_requires_current_authenticated_authorized_producer() {
    let f = Fixture::new(false, false);
    assert!(f.eval(&f.provider(), NOW).eligible);
    for error in [AuthorityError::Untrusted, AuthorityError::Unavailable] {
        let mut a = f.provider();
        a.producer = Err(error);
        assert!(!f.eval(&a, NOW).eligible);
    }
    let mut a = f.provider();
    a.producer.as_mut().unwrap().principal = "stranger".into();
    assert_eq!(f.eval(&a, NOW).code, EligibilityCode::UnauthorizedProducer);
    let mut a = f.provider();
    a.producer.as_mut().unwrap().envelope_digest = digest(b"forged");
    assert!(!f.eval(&a, NOW).eligible);
    let mut a = f.provider();
    a.producer.as_mut().unwrap().producer.version = "forged".into();
    assert!(!f.eval(&a, NOW).eligible);
}
#[test]
fn changed_binding_is_stale() {
    let f = Fixture::new(false, false);
    let original = serde_json::to_value(&f.e).unwrap();
    for (path, value) in [
        ("/binding/repoId", json!("other")),
        ("/binding/taskId", json!("other")),
        ("/binding/worktreeId", json!("other")),
        ("/binding/requirementIds", json!(["other"])),
        ("/binding/candidateOid", json!("e".repeat(40))),
        ("/binding/baseOid", json!("e".repeat(40))),
        ("/binding/mergeGroupId", json!("group")),
        ("/binding/sourceSnapshotDigest", json!(digest(b"changed"))),
        ("/binding/baselineDigest", json!(digest(b"changed"))),
        ("/producer/analyzerId", json!("other")),
        ("/producer/analyzerVersion", json!("other")),
        ("/producer/version", json!("other")),
        ("/producer/guard", json!("other")),
        ("/coverage/requiredScopes", json!(["other"])),
        ("/artifacts/contract/digest", json!(digest(b"changed"))),
    ] {
        let mut f = Fixture::new(false, false);
        let mut v = original.clone();
        *v.pointer_mut(path).unwrap() = value;
        f.e = serde_json::from_value(v).unwrap();
        assert!(!f.eval(&f.provider(), NOW).eligible, "{path}");
    }
}
#[test]
fn approval_cannot_rewrite_report() {
    let (f, a) = Fixture::approved();
    let before = serde_json::to_vec(&f.e).unwrap();
    let result = f.eval(&a, NOW);
    assert!(result.eligible);
    assert_eq!(result.technical_decision, Some(Decision::RequireApproval));
    assert_eq!(serde_json::to_vec(&f.e).unwrap(), before);
    let f = Fixture::new(true, false);
    assert_eq!(
        f.eval(&f.provider(), NOW).code,
        EligibilityCode::MissingApproval
    );
    let (f, mut a) = Fixture::approved();
    a.approvals.remove("approval-b");
    assert!(!f.eval(&a, NOW).eligible);
    for variant in 0..9 {
        let (f, mut a) = Fixture::approved();
        let r = a.approvals.get_mut("approval-a").unwrap().as_mut().unwrap();
        match variant {
            0 => r.principal = "stranger".into(),
            1 => r.action = "deploy".into(),
            2 => r.purpose = "other".into(),
            3 => r.binding.candidate_oid = "e".repeat(40),
            4 => r.contract_digest = digest(b"changed"),
            5 => r.validity.revoked = true,
            6 => r.validity.expires_at = NOW,
            7 => r.validity.issued_at = NOW + 1,
            _ => r.binding.requirement_ids = vec!["other".into()],
        };
        assert!(!f.eval(&a, NOW).eligible, "variant {variant}");
    }
    let (mut f, a) = Fixture::approved();
    f.p.approval_principals.get_mut("security").unwrap().clear();
    assert!(!f.eval(&a, NOW).eligible);
}
#[test]
fn partial_error_cancelled_and_technical_block_never_qualify() {
    let f = Fixture::new(true, true);
    assert_eq!(f.eval(&f.provider(), NOW).code, EligibilityCode::Incomplete);
    for status in [RunStatus::Error, RunStatus::Cancelled] {
        let mut f = Fixture::new(false, false);
        f.e.run_status = status;
        f.e.decision = None;
        f.e.artifacts.report = None;
        f.e.diagnostics.push(Diagnostic {
            code: "failed".into(),
            message: "secret".into(),
            retryable: false,
            source: None,
        });
        assert_eq!(
            f.eval(&f.provider(), NOW).code,
            EligibilityCode::ExecutionFailed
        );
    }
    let mut f = Fixture::new(true, false);
    let text = String::from_utf8(f.cb.clone())
        .unwrap()
        .replace("review", "enforce");
    f.cb = text.into_bytes();
    let report = guardengine::evaluate(
        &guardengine::load_contract_yaml(&f.cb).unwrap(),
        &guardengine::load_facts_json(&f.fb).unwrap(),
    )
    .unwrap();
    f.rb = serde_json::to_vec(&report).unwrap();
    f.e.decision = Some(Decision::Block);
    f.e.artifacts.contract.as_mut().unwrap().digest = digest(&f.cb);
    f.p.contract_digest = digest(&f.cb);
    f.e.artifacts.report.as_mut().unwrap().digest = digest(&f.rb);
    assert_eq!(
        f.eval(&f.provider(), NOW).code,
        EligibilityCode::TechnicalBlock
    );
}
#[test]
fn expiry_endpoints_and_per_call_revocation_are_enforced() {
    let f = Fixture::new(false, false);
    let mut a = f.provider();
    assert!(f.eval(&a, NOW).eligible);
    assert!(!f.eval(&a, NOW - 2).eligible); // finish is in future
    assert!(!f.eval(&a, NOW + 8).eligible);
    assert!(f.eval(&a, NOW + 7).eligible);
    a.producer.as_mut().unwrap().validity.revoked = true;
    assert!(!f.eval(&a, NOW).eligible);
    for (issued, expires) in [(NOW + 1, NOW + 8), (NOW - 1, NOW), (NOW, NOW)] {
        let mut a = f.provider();
        a.producer.as_mut().unwrap().validity = Validity {
            issued_at: issued,
            expires_at: expires,
            revoked: false,
        };
        assert!(!f.eval(&a, NOW).eligible);
    }
    let mut a = f.provider();
    a.producer.as_mut().unwrap().validity.issued_at = NOW;
    assert!(f.eval(&a, NOW).eligible);
}
#[test]
fn artifacts_are_reverified_and_audit_omits_freeform_secrets() {
    let mut f = Fixture::new(false, false);
    let a = f.provider();
    assert!(f.eval(&a, NOW).eligible);
    f.rb.push(b' ');
    assert_eq!(f.eval(&a, NOW).code, EligibilityCode::InvalidEvidence);
    f.rb.pop();
    f.e.artifacts.report = None;
    assert!(!f.eval(&f.provider(), NOW).eligible);
    let (mut f, _) = Fixture::approved();
    f.e.diagnostics.push(Diagnostic {
        code: "user code secret".into(),
        message: "secret diagnostic".into(),
        source: Some("secret path".into()),
        retryable: false,
    });
    let mut a = f.provider();
    a.approvals
        .insert("approval-a".into(), Ok(f.approval("security")));
    a.approvals
        .insert("approval-b".into(), Ok(f.approval("release")));
    let result = f.eval(&a, NOW);
    assert!(result.eligible);
    let text = serde_json::to_string(&result.audit).unwrap();
    assert!(!text.contains("secret"));
    assert_eq!(result.audit.actor_digest, Some(digest(b"ci")));
    assert_eq!(result.audit.action_digest, digest(b"merge"));
    assert_eq!(result.audit.cause_digest, Some(digest(b"secret cause")));
    assert_eq!(result.audit.approver_digests, vec![digest(b"reviewer")]);
    assert_eq!(result.audit.timestamp, NOW);
    assert_eq!(
        result.audit.envelope_digest,
        digest(&serde_json::to_vec(&f.e).unwrap())
    );
    assert_eq!(
        result.audit.binding_digest,
        digest(&serde_json::to_vec(&f.e.binding).unwrap())
    );
}
#[test]
fn booleans_and_native_only_cannot_supply_authority() {
    let f = Fixture::new(false, false);
    let mut v = serde_json::to_value(&f.e).unwrap();
    v["trusted"] = json!(true);
    assert!(
        load_envelope_json(
            &serde_json::to_vec(&v).unwrap(),
            EvidenceProfile::EngineBacked
        )
        .is_err()
    );
    let mut f = Fixture::new(false, false);
    f.e.artifacts.contract = None;
    f.e.artifacts.facts = None;
    f.e.artifacts.report = None;
    assert!(!f.eval(&f.provider(), NOW).eligible);
    let mut f = Fixture::new(false, false);
    f.p.producer_principals.clear();
    assert_eq!(
        f.eval(&f.provider(), NOW).code,
        EligibilityCode::InvalidPolicy
    );
}

#[test]
fn purpose_coverage_unavailability_and_fractional_time_fail_closed() {
    let (f, mut a) = Fixture::approved();
    a.approvals
        .insert("approval-b".into(), Ok(f.approval("security")));
    assert_eq!(f.eval(&a, NOW).code, EligibilityCode::MissingApproval);
    a.approvals
        .insert("approval-b".into(), Err(AuthorityError::Unavailable));
    assert_eq!(f.eval(&a, NOW).code, EligibilityCode::ProviderUnavailable);
    let mut f = Fixture::new(false, false);
    f.e.finished_at = "2026-10-09T10:00:02.1Z".into();
    assert_eq!(f.eval(&f.provider(), NOW).code, EligibilityCode::Stale);
    assert!(f.eval(&f.provider(), NOW + 1).eligible);
    f.e.expires_at = Some("2026-10-09T10:00:03.1Z".into());
    assert!(f.eval(&f.provider(), NOW + 1).eligible);
    assert_eq!(f.eval(&f.provider(), NOW + 2).code, EligibilityCode::Stale);
}

#[test]
fn content_key_binds_configuration_and_authorization_not_attempt_id() {
    let (mut f, _) = Fixture::approved();
    let key = f.p.content_digest();
    assert_eq!(key, f.p.clone().content_digest());
    let result = f.eval(&f.provider(), NOW);
    assert_eq!(result.audit.content_digest, key);
    f.e.run_id = "another-attempt".into();
    assert_eq!(key, f.p.content_digest());
    for variant in 0..8 {
        let mut p = f.p.clone();
        match variant {
            0 => p.binding.candidate_oid = "f".repeat(40),
            1 => p.contract_digest = digest(b"policy"),
            2 => p.producer.analyzer_version = "next".into(),
            3 => p.required_scopes = vec!["different".into()],
            4 => p.action = "deploy".into(),
            5 => {
                p.producer_principals.insert("other-ci".into());
            }
            6 => {
                p.approval_principals
                    .get_mut("security")
                    .unwrap()
                    .insert("other-reviewer".into());
            }
            _ => {
                p.approval_principals
                    .insert("other-purpose".into(), ["reviewer".into()].into());
            }
        }
        assert_ne!(key, p.content_digest(), "variant {variant}");
    }
}

#[test]
fn public_approval_validator_matches_exact_purpose_binding_and_validity() {
    let (f, _) = Fixture::approved();
    let record = f.approval("security");
    assert_eq!(
        validate_approval_record(&record, &f.p, "security", NOW),
        Ok(())
    );
    assert_eq!(
        validate_approval_record(&record, &f.p, "release", NOW),
        Err(EligibilityCode::InvalidApproval)
    );
    for field in 0..16 {
        let mut changed = record.clone();
        match field {
            0 => changed.principal = "stranger".into(),
            1 => changed.purpose = "release".into(),
            2 => changed.action = "deploy".into(),
            3 => changed.contract_digest = digest(b"other"),
            4 => changed.binding.repo_id = "other".into(),
            5 => changed.binding.task_id = "other".into(),
            6 => changed.binding.worktree_id = "other".into(),
            7 => changed.binding.requirement_ids = vec!["other".into()],
            8 => changed.binding.candidate_oid = "f".repeat(40),
            9 => changed.binding.base_oid = "f".repeat(40),
            10 => changed.binding.merge_group_id = Some("other".into()),
            11 => changed.binding.source_snapshot_digest = digest(b"other"),
            12 => changed.binding.baseline_digest = Some(digest(b"other")),
            13 => changed.validity.revoked = true,
            14 => changed.validity.issued_at = NOW + 1,
            _ => changed.validity.expires_at = NOW,
        }
        assert_eq!(
            validate_approval_record(&changed, &f.p, "security", NOW),
            Err(EligibilityCode::InvalidApproval),
            "field {field}"
        );
    }
    let mut endpoints = record.clone();
    endpoints.validity.issued_at = NOW;
    endpoints.validity.expires_at = NOW + 1;
    assert_eq!(
        validate_approval_record(&endpoints, &f.p, "security", NOW),
        Ok(())
    );
    assert_eq!(
        validate_approval_record(&endpoints, &f.p, "security", NOW + 1),
        Err(EligibilityCode::InvalidApproval)
    );
    endpoints.validity.expires_at = NOW;
    assert!(validate_approval_record(&endpoints, &f.p, "security", NOW).is_err());
    let mut policy = f.p.clone();
    policy.approval_principals.remove("security");
    assert_eq!(
        validate_approval_record(&record, &policy, "security", NOW),
        Err(EligibilityCode::InvalidApproval)
    );
}

#[test]
fn public_approval_validator_rejects_malformed_protected_policy() {
    let (f, _) = Fixture::approved();
    let record = f.approval("security");
    for variant in 0..4 {
        let mut policy = f.p.clone();
        match variant {
            0 => policy.action = " ".into(),
            1 => policy.producer_principals.clear(),
            2 => policy
                .approval_principals
                .get_mut("security")
                .unwrap()
                .clear(),
            _ => {
                policy
                    .approval_principals
                    .get_mut("security")
                    .unwrap()
                    .insert(" ".into());
            }
        }
        assert_eq!(
            validate_approval_record(&record, &policy, "security", NOW),
            Err(EligibilityCode::InvalidPolicy)
        );
    }
}
