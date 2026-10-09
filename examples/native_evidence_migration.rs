//! Existing API migration demonstration, never a production authority provider.
use guardengine::integration::{
    EvidenceProfile, GuardRunEnvelope,
    attempt_store::{AttemptRecord, AttemptStore, InMemoryAttemptStore, Target},
    eligibility::{
        ApprovalRecord, ArtifactBytes, AuthorityError, AuthorityProvider, EligibilityPolicy,
        ProducerRecord, evaluate_eligibility,
    },
    load_envelope_json, verify_engine_artifacts,
};
use guardengine::{evaluate, load_contract_yaml, load_facts_json};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

// Intentionally supplies no authentication. Never enroll arbitrary uploaded evidence.
struct UnavailableAuthority;
impl AuthorityProvider for UnavailableAuthority {
    fn verify_producer(
        &self,
        _: &GuardRunEnvelope,
        _: &str,
    ) -> Result<ProducerRecord, AuthorityError> {
        Err(AuthorityError::Unavailable)
    }
    fn verify_approval(&self, _: &str) -> Result<ApprovalRecord, AuthorityError> {
        Err(AuthorityError::Unavailable)
    }
}
fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}
fn observation() -> Value {
    // Frozen corpus has actual native artifacts but synthetic controller bindings.
    // Production controllers freeze their own policy/binding; never derive trust
    // from a candidate envelope as this closed documentation fixture does.
    let contract =
        include_bytes!("../tests/fixtures/consumer-corpus/archguard-allow/contract.yaml");
    let facts = include_bytes!("../tests/fixtures/consumer-corpus/archguard-allow/facts.json");
    let report = include_bytes!("../tests/fixtures/consumer-corpus/archguard-allow/report.json");
    let original_digest = digest(report);
    let native = evaluate(
        &load_contract_yaml(contract).unwrap(),
        &load_facts_json(facts).unwrap(),
    )
    .unwrap();
    let envelope = load_envelope_json(
        include_bytes!("../tests/fixtures/consumer-corpus/archguard-allow/envelope.json"),
        EvidenceProfile::EngineBacked,
    )
    .unwrap();
    let verified = verify_engine_artifacts(&envelope, contract, facts, report).unwrap();
    assert_eq!(native, verified);
    let policy = EligibilityPolicy {
        binding: envelope.binding.clone(),
        producer: envelope.producer.clone(),
        required_scopes: envelope.coverage.required_scopes.clone(),
        contract_digest: digest(contract),
        action: "documented-local-consumption".into(),
        producer_principals: ["controller-owned-principal".into()].into(),
        approval_principals: BTreeMap::new(),
    };
    let now = 1_791_540_002; // Fixed fixture clock: 2026-10-09T10:00:02Z.
    let unavailable = evaluate_eligibility(
        &envelope,
        ArtifactBytes {
            contract,
            facts,
            report,
        },
        &policy,
        &UnavailableAuthority,
        now,
        Some("secret-predecessor"),
    );
    // Required-original deletion/expiry is represented by its actual absence,
    // never by retaining a previous green decision as authorization.
    let missing = evaluate_eligibility(
        &envelope,
        ArtifactBytes {
            contract,
            facts,
            report: &[],
        },
        &policy,
        &UnavailableAuthority,
        now,
        None,
    );
    assert!(!unavailable.eligible && !missing.eligible);
    let target = Target {
        repo_id: policy.binding.repo_id.clone(),
        task_id: policy.binding.task_id.clone(),
        requirement_ids: policy.binding.requirement_ids.clone(),
    };
    let mut store = InMemoryAttemptStore::default();
    let generation = store
        .advance(target.clone(), 0, policy.content_digest(), "first".into())
        .unwrap();
    let first = AttemptRecord {
        run_id: "first".into(),
        target: target.clone(),
        generation,
        content_digest: policy.content_digest(),
        envelope_digest: digest(&serde_json::to_vec(&envelope).unwrap()),
        eligible: unavailable.eligible,
    };
    store.append(first.clone()).unwrap();
    store.publish("first").unwrap(); // Current denied observation, not a permit.
    let next = store
        .advance(
            target.clone(),
            generation,
            policy.content_digest(),
            "recheck".into(),
        )
        .unwrap();
    let recheck_clears_current = store.current(&target).is_none();
    store
        .append(AttemptRecord {
            run_id: "recheck".into(),
            generation: next,
            eligible: missing.eligible,
            ..first.clone()
        })
        .unwrap();
    json!({"native_decision": native.decision, "without_authority": unavailable.code,
        "missing_report": missing.code, "history_records": store.history(&target).len(),
        "prior_record_unchanged": store.history(&target)[0] == &first,
        "recheck_clears_current": recheck_clears_current,
        "native_bytes_unchanged": digest(report) == original_digest,
        "native_report_digest": original_digest,
        "audit": [unavailable.audit, missing.audit],
        "execution_authorized": false})
}
fn main() {
    println!("{}", observation());
}
#[cfg(test)]
mod tests {
    #[test]
    fn native_allow_never_substitutes_for_authority_or_required_originals() {
        let result = super::observation();
        assert_eq!(result["native_decision"], "ALLOW");
        assert_eq!(result["without_authority"], "provider_unavailable");
        assert_eq!(result["missing_report"], "invalid_evidence");
        assert_eq!(result["history_records"], 2);
        assert_eq!(result["prior_record_unchanged"], true);
        assert_eq!(result["recheck_clears_current"], true);
        assert_eq!(result["native_bytes_unchanged"], true);
        assert_eq!(result["execution_authorized"], false);
        assert!(!result.to_string().contains("secret-predecessor"));
    }
}
