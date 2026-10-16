use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::GuardError;
use crate::protocol::{
    API_VERSION_V2, AnalyzerIdentity, Completeness, Enforcement, GuardContract, GuardFact,
    GuardFacts, GuardSubject,
};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RuleStatus {
    Pass,
    Fail,
    ReviewRequired,
    Indeterminate,
    NotApplicable,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Decision {
    Allow,
    Block,
    RequireApproval,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuleEvaluation {
    pub rule_id: String,
    pub status: RuleStatus,
    pub enforcement: Enforcement,
    pub matched_facts: Vec<GuardFact>,
    pub detail: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GuardReport {
    #[serde(rename = "apiVersion")]
    pub api_version: String,
    pub kind: String,
    pub evaluation_id: String,
    pub engine_version: String,
    pub contract_id: String,
    pub contract_revision: String,
    pub contract_digest: String,
    pub facts_digest: String,
    pub analyzer: AnalyzerIdentity,
    pub subject: GuardSubject,
    pub evaluations: Vec<RuleEvaluation>,
    pub decision: Decision,
    pub signed: bool,
}

/// A stable hexadecimal SHA-256 digest over a canonical serialization of the provided struct.
/// Struct field order is deterministic; caller-supplied fact ordering is normalized before hashing.
pub fn digest_json<T: Serialize>(value: &T) -> Result<String, GuardError> {
    let bytes =
        serde_json::to_vec(value).map_err(|error| GuardError::Serialization(error.to_string()))?;
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    Ok(format!("sha256:{:x}", hasher.finalize()))
}

pub fn evaluate(
    contract: &GuardContract,
    fact_set: &GuardFacts,
) -> Result<GuardReport, GuardError> {
    contract.validate()?;
    fact_set.validate()?;

    let mut canonical_facts = fact_set.clone();
    canonical_facts.facts.sort();
    canonical_facts.facts.dedup();
    if let Some(coverage) = &mut canonical_facts.coverage {
        coverage.entities.sort();
        coverage.predicates.sort();
    }

    let contract_digest = digest_json(contract)?;
    let facts_digest = digest_json(&canonical_facts)?;
    let evaluation_id = digest_json(&(
        &contract.api_version,
        env!("CARGO_PKG_VERSION"),
        &contract_digest,
        &facts_digest,
    ))?;

    let mut evaluations = Vec::with_capacity(contract.spec.rules.len());
    for rule in &contract.spec.rules {
        let (violated, matched) =
            crate::graph_evaluation::violation(&rule.assertion, &canonical_facts.facts);
        let coverage_problem = crate::graph_evaluation::coverage_problem(
            &rule.assertion,
            &canonical_facts,
            contract.api_version == API_VERSION_V2,
        );
        let (status, detail) = if canonical_facts.completeness == Completeness::Partial {
            (
                RuleStatus::Indeterminate,
                format!(
                    "analyzer did not provide a complete fact set: {}",
                    canonical_facts.diagnostics.join("; ")
                ),
            )
        } else if let Some(problem) = coverage_problem {
            (RuleStatus::Indeterminate, problem)
        } else if !violated {
            (
                RuleStatus::Pass,
                "constraint satisfied within declared scope".to_owned(),
            )
        } else {
            match rule.enforcement {
                Enforcement::Enforce => (
                    RuleStatus::Fail,
                    format!("constraint violated; {} witness edge(s)", matched.len()),
                ),
                Enforcement::Review => (
                    RuleStatus::ReviewRequired,
                    format!("{} relation(s) need approval", matched.len()),
                ),
                Enforcement::Advise => (
                    RuleStatus::Pass,
                    format!("advisory: found {} matching relation(s)", matched.len()),
                ),
            }
        };
        evaluations.push(RuleEvaluation {
            rule_id: rule.id.clone(),
            status,
            enforcement: rule.enforcement.clone(),
            matched_facts: matched,
            detail,
        });
    }

    let decision = if evaluations
        .iter()
        .any(|item| matches!(item.status, RuleStatus::Fail | RuleStatus::Indeterminate))
    {
        Decision::Block
    } else if evaluations
        .iter()
        .any(|item| matches!(item.status, RuleStatus::ReviewRequired))
    {
        Decision::RequireApproval
    } else {
        Decision::Allow
    };

    Ok(GuardReport {
        api_version: contract.api_version.clone(),
        kind: "GuardReport".into(),
        evaluation_id,
        engine_version: env!("CARGO_PKG_VERSION").into(),
        contract_id: contract.metadata.id.clone(),
        contract_revision: contract.metadata.revision.clone(),
        contract_digest,
        facts_digest,
        analyzer: fact_set.analyzer.clone(),
        subject: fact_set.subject.clone(),
        evaluations,
        decision,
        // Local reports are UNSIGNED by design. A trusted CI can later issue attestations.
        signed: false,
    })
}

/// Recompute the entire report; self-described digests are never accepted as verification proof.
pub fn verify_report(
    report: &GuardReport,
    contract: &GuardContract,
    facts: &GuardFacts,
) -> Result<bool, GuardError> {
    Ok(&evaluate(contract, facts)? == report)
}
