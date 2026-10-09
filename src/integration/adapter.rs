use super::*;
/// Verify byte integrity, native report recomputation and envelope/report agreement.
/// Successful verification is not producer authentication or Git object proof.
pub fn verify_engine_artifacts(
    envelope: &GuardRunEnvelope,
    contract_yaml: &[u8],
    facts_json: &[u8],
    report_json: &[u8],
) -> Result<crate::GuardReport, IntegrationError> {
    use sha2::{Digest, Sha256};
    envelope.validate(EvidenceProfile::EngineBacked)?;
    if envelope.run_status != RunStatus::Completed {
        return fail("only completed evidence can be verified");
    }
    for (reference, bytes) in [
        (&envelope.artifacts.contract, contract_yaml),
        (&envelope.artifacts.facts, facts_json),
        (&envelope.artifacts.report, report_json),
    ] {
        if bytes.len() > MAX_ARTIFACT_BYTES {
            return fail("artifact size exceeded");
        }
        let reference = reference
            .as_ref()
            .ok_or_else(|| IntegrationError("missing artifact reference".into()))?;
        if reference.digest != format!("sha256:{:x}", Sha256::digest(bytes)) {
            return fail("artifact byte digest mismatch");
        }
    }
    let contract =
        crate::load_contract_yaml(contract_yaml).map_err(|e| IntegrationError(e.to_string()))?;
    let facts = crate::load_facts_json(facts_json).map_err(|e| IntegrationError(e.to_string()))?;
    let report: crate::GuardReport =
        serde_json::from_slice(report_json).map_err(|e| IntegrationError(e.to_string()))?;
    if !crate::verify_report(&report, &contract, &facts)
        .map_err(|e| IntegrationError(e.to_string()))?
    {
        return fail("report differs from recomputation");
    }
    if report.analyzer.id != envelope.producer.analyzer_id
        || report.analyzer.version != envelope.producer.analyzer_version
        || report.subject.snapshot_digest != envelope.binding.source_snapshot_digest
        || envelope.decision.as_ref() != Some(&report.decision)
    {
        return fail("envelope and engine report disagree");
    }
    if (facts.completeness == crate::Completeness::Complete)
        != (envelope.coverage.status == CoverageStatus::Complete)
    {
        return fail("envelope and analyzer completeness disagree");
    }
    Ok(report)
}
