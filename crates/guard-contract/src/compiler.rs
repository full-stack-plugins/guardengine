use crate::diagnostic::issue;
use crate::{
    ApprovalAuthority, CapabilityProbe, CompileRequest, CompiledContract, ContractCompilation,
    LayerIdentity, LayerLevel, SchemaValidator, VerifierKind, VerifierObservation,
    VerifierRegistry, snapshot_digest,
};
use guard_protocol::{
    ContractObligation, ContractSnapshot, Digest, Identifier, ProtocolDocument, RuleDefinition,
};
use serde_json::json;
use sha2::{Digest as Hasher, Sha256};
use std::collections::{BTreeMap, BTreeSet};

/// 只读组合已批准的组织/项目/任务约束，缺任一准备条件则返回blocked并保留规则。
/// 仅调用显式注入的只读权威/能力端口，不运行OPA、验证器、安装器、Git或文件遍历。
pub fn compile(
    request: &CompileRequest,
    registry: &VerifierRegistry,
    probe: &dyn CapabilityProbe,
    authority: Option<&dyn ApprovalAuthority>,
) -> Result<ContractCompilation, String> {
    let schema = SchemaValidator::protocol()?;
    let mut result = ContractCompilation {
        diagnostics: Vec::new(),
        rules: Vec::new(),
        obligations: Vec::new(),
        compiled: None,
    };
    let errors = &mut result.diagnostics;
    if authority.is_none() {
        issue(
            errors,
            "approval_authority_missing",
            None,
            None,
            "independent read-only approval authority is required",
        );
    }
    if request.context.authorization_refs.is_empty() {
        issue(
            errors,
            "authorization_missing",
            None,
            None,
            "existing scoped authorization reference is required",
        );
    }
    let refs = request
        .context
        .authorization_refs
        .iter()
        .collect::<BTreeSet<_>>();
    if refs.len() != request.context.authorization_refs.len() {
        issue(
            errors,
            "authorization_duplicate",
            None,
            None,
            "duplicate authorization reference",
        );
    }
    if request.layers.len() > 8 {
        issue(
            errors,
            "layer_budget_exceeded",
            None,
            None,
            "at most eight declared layers may be diagnosed",
        );
        return Ok(result);
    }
    let mut approvals = Vec::new();
    let mut layers = BTreeMap::<LayerLevel, ContractSnapshot>::new();
    for layer in &request.layers {
        if layers.contains_key(&layer.level) {
            issue(
                errors,
                "duplicate_layer",
                Some(layer.level),
                None,
                "each level must occur exactly once",
            );
            continue;
        }
        match schema.validate_bytes(&layer.document) {
            Err(_) => {
                issue(
                    errors,
                    "protocol_json_invalid",
                    Some(layer.level),
                    None,
                    "strict JSON parsing failed",
                );
                continue;
            }
            Ok(issues) if !issues.is_empty() => {
                for problem in issues {
                    issue(
                        errors,
                        "protocol_schema_invalid",
                        Some(layer.level),
                        None,
                        &format!("{} at {}", problem.schema_path, problem.instance_path),
                    );
                }
                continue;
            }
            Ok(_) => {}
        }
        let parsed = match guard_protocol::decode(&layer.document) {
            Ok(value) => value,
            Err(_) => {
                issue(
                    errors,
                    "protocol_semantics_invalid",
                    Some(layer.level),
                    None,
                    "closed protocol semantic validation failed",
                );
                continue;
            }
        };
        let ProtocolDocument::ContractSnapshot(snapshot) = parsed.document() else {
            issue(
                errors,
                "wrong_document_type",
                Some(layer.level),
                None,
                "ContractSnapshot is required",
            );
            continue;
        };
        let calculated = snapshot_digest(snapshot)?;
        if calculated != snapshot.digest {
            issue(
                errors,
                "snapshot_digest_mismatch",
                Some(layer.level),
                None,
                "declared digest does not match actual canonical snapshot content",
            );
        }
        let total = layer
            .sources
            .iter()
            .try_fold(0usize, |sum, s| sum.checked_add(s.bytes.len()));
        if total.is_none_or(|n| n > 32 * 1024 * 1024) || layer.sources.len() > 8192 {
            issue(
                errors,
                "source_budget_exceeded",
                Some(layer.level),
                None,
                "source material exceeds bounded read-only input budget",
            );
        } else {
            for reference in &snapshot.source_refs {
                let materials = layer
                    .sources
                    .iter()
                    .filter(|s| s.source == *reference)
                    .collect::<Vec<_>>();
                if materials.len() != 1 {
                    issue(
                        errors,
                        "source_missing_or_duplicate",
                        Some(layer.level),
                        None,
                        "each exact source reference requires exactly one material",
                    );
                    continue;
                }
                let bytes = &materials[0].bytes;
                if bytes.len() > 4 * 1024 * 1024 {
                    issue(
                        errors,
                        "source_budget_exceeded",
                        Some(layer.level),
                        None,
                        "source exceeds 4 MiB",
                    );
                    continue;
                }
                let actual = Digest::new(format!("sha256:{:x}", Sha256::digest(bytes)))?;
                if actual != reference.content_digest {
                    issue(
                        errors,
                        "source_digest_mismatch",
                        Some(layer.level),
                        None,
                        "raw source bytes do not match approved source digest",
                    );
                }
            }
            if layer
                .sources
                .iter()
                .any(|s| !snapshot.source_refs.contains(&s.source))
            {
                issue(
                    errors,
                    "source_unreferenced",
                    Some(layer.level),
                    None,
                    "unreferenced source material cannot establish provenance",
                );
            }
        }
        if let Some(authority) = authority {
            match authority.verify_layer(
                &request.context,
                layer.level,
                &snapshot.approval_ref,
                &calculated,
                request.action,
            ) {
                Ok(proof)
                    if proof.context == request.context
                        && proof.contract_digest == calculated
                        && proof.level == layer.level
                        && proof.approval_ref == snapshot.approval_ref
                        && proof.action == request.action =>
                {
                    approvals.push(proof);
                }
                Ok(_) => issue(
                    errors,
                    "approval_binding_mismatch",
                    Some(layer.level),
                    None,
                    "approval does not bind exact context/action/layer/digest/reference",
                ),
                Err(_) => issue(
                    errors,
                    "snapshot_not_approved",
                    Some(layer.level),
                    None,
                    "independent authority rejected the snapshot approval",
                ),
            }
        }
        layers.insert(layer.level, snapshot.clone());
    }
    for level in [
        LayerLevel::Organization,
        LayerLevel::Project,
        LayerLevel::Task,
    ] {
        if !layers.contains_key(&level) {
            issue(
                errors,
                "required_layer_missing",
                Some(level),
                None,
                "organization, project and task layers are all required",
            );
        }
    }
    let mut rules = BTreeMap::<Identifier, (LayerLevel, RuleDefinition)>::new();
    let mut obligations = BTreeMap::<Identifier, ContractObligation>::new();
    let mut identities = Vec::new();
    for (level, snapshot) in &layers {
        identities.push(LayerIdentity {
            level: *level,
            contract_id: snapshot.contract_id.clone(),
            version: snapshot.version.clone(),
            digest: snapshot.digest.clone(),
            approval_ref: snapshot.approval_ref.clone(),
        });
        for rule in &snapshot.rules {
            match rules.get(&rule.rule_id) {
                Some((origin, existing)) if existing != rule => issue(
                    errors,
                    if *origin == LayerLevel::Organization {
                        "organization_rule_locked"
                    } else {
                        "rule_definition_conflict"
                    },
                    Some(*level),
                    Some(&rule.rule_id),
                    "same rule ID has a different approved definition; no override or weakening is allowed",
                ),
                Some(_) => {}
                None => {
                    rules.insert(rule.rule_id.clone(), (*level, rule.clone()));
                }
            }
        }
        for obligation in &snapshot.obligations {
            match obligations.get(&obligation.obligation_id) {
                Some(existing) if existing != obligation => issue(
                    errors,
                    "obligation_definition_conflict",
                    Some(*level),
                    Some(&obligation.rule_id),
                    "same obligation ID has a different required target/evidence definition",
                ),
                Some(_) => {}
                None => {
                    obligations.insert(obligation.obligation_id.clone(), obligation.clone());
                }
            }
        }
    }
    result.rules = rules.values().map(|(_, r)| r.clone()).collect();
    result.obligations = obligations.values().cloned().collect();
    if result.rules.is_empty() {
        issue(
            errors,
            "empty_effective_contract",
            None,
            None,
            "empty rule sets never mean a ready contract",
        );
    }
    let mut used = BTreeSet::new();
    for rule in &result.rules {
        used.insert(rule.verifier_id.clone());
        if !result.obligations.iter().any(|o| o.rule_id == rule.rule_id) {
            issue(
                errors,
                "rule_without_obligation",
                None,
                Some(&rule.rule_id),
                "every effective rule requires a planned obligation",
            );
        }
        match registry.entries.get(&rule.verifier_id) {
            None => issue(
                errors,
                "verifier_missing",
                None,
                Some(&rule.rule_id),
                "rule references an unregistered verifier",
            ),
            Some(entry) => {
                if entry.pin.kind != VerifierKind::Domain || entry.pin.owner != Some(rule.owner) {
                    issue(
                        errors,
                        "verifier_owner_mismatch",
                        None,
                        Some(&rule.rule_id),
                        "professional verifier owner/kind does not match rule owner",
                    );
                }
                for problem in entry.parameters.validate(&rule.parameters) {
                    issue(
                        errors,
                        "verifier_parameters_invalid",
                        None,
                        Some(&rule.rule_id),
                        &format!("{} at {}", problem.schema_path, problem.instance_path),
                    );
                }
                if rule
                    .required_evidence
                    .iter()
                    .any(|e| !entry.pin.evidence_types.contains(e))
                {
                    issue(
                        errors,
                        "verifier_evidence_unsupported",
                        None,
                        Some(&rule.rule_id),
                        "verifier cannot produce a required evidence kind",
                    );
                }
            }
        }
    }
    for obligation in &result.obligations {
        match rules.get(&obligation.rule_id) {
            None => issue(
                errors,
                "obligation_rule_missing",
                None,
                Some(&obligation.rule_id),
                "obligation references no effective rule",
            ),
            Some((_, rule)) => {
                if rule.verifier_id != obligation.verifier_id {
                    issue(
                        errors,
                        "obligation_verifier_mismatch",
                        None,
                        Some(&rule.rule_id),
                        "obligation cannot switch the approved rule verifier",
                    );
                }
                if rule
                    .required_evidence
                    .iter()
                    .any(|e| !obligation.required_evidence.contains(e))
                {
                    issue(
                        errors,
                        "obligation_evidence_weakened",
                        None,
                        Some(&rule.rule_id),
                        "required rule evidence cannot be removed from obligation",
                    );
                }
            }
        }
        if obligation
            .targets
            .iter()
            .any(|t| t.repository_id != request.context.repository_id)
        {
            issue(
                errors,
                "target_scope_mismatch",
                None,
                Some(&obligation.rule_id),
                "obligation target is outside the authorized repository context",
            );
        }
    }
    used.insert(registry.policy_engine_id.clone());
    let mut pins = Vec::new();
    for id in used {
        let Some(entry) = registry.entries.get(&id) else {
            if id == registry.policy_engine_id {
                issue(
                    errors,
                    "policy_engine_missing",
                    None,
                    None,
                    "configured OPA policy engine is not registered; no execution or installation attempted",
                );
            }
            continue;
        };
        if id == registry.policy_engine_id && entry.pin.kind != VerifierKind::PolicyEngine {
            issue(
                errors,
                "policy_engine_kind_mismatch",
                None,
                None,
                "configured policy engine cannot be a professional verifier",
            );
        }
        match probe.observe(&entry.pin) {
            VerifierObservation::Available(observed) if observed == entry.pin => {}
            VerifierObservation::Available(_) => issue(
                errors,
                "verifier_identity_mismatch",
                None,
                None,
                id.as_str(),
            ),
            VerifierObservation::Missing(_) => issue(
                errors,
                "verifier_unavailable",
                None,
                None,
                &format!("{}: not installed; preparation required", id.as_str()),
            ),
            VerifierObservation::Unsupported(_) => issue(
                errors,
                "verifier_platform_unsupported",
                None,
                None,
                id.as_str(),
            ),
            VerifierObservation::Unknown(_) => issue(
                errors,
                "verifier_availability_unknown",
                None,
                None,
                id.as_str(),
            ),
        }
        pins.push(entry.pin.clone());
    }
    let mut source_map = BTreeMap::new();
    for snapshot in layers.values() {
        for source in &snapshot.source_refs {
            source_map.insert(
                serde_json::to_vec(source).map_err(|e| e.to_string())?,
                source.clone(),
            );
        }
    }
    let sources = source_map.into_values().collect::<Vec<_>>();
    approvals.sort_by_key(|a| a.level);
    let approvers = approvals
        .iter()
        .map(|a: &crate::ApprovalAttestation| json!({"layer":a.level,"authorityId":a.authority_id}))
        .collect::<Vec<_>>();
    let content = json!({"schemaVersion":"1.0.0","documentType":"CompiledContract",
        "context":request.context,"action":request.action,"layers":identities,"rules":result.rules,
        "obligations":result.obligations,"verifiers":pins,"protocolSchemaDigest":schema.digest(),"sourceRefs":sources,"approvalAuthorities":approvers});
    let digest =
        guard_protocol::digest_json(&serde_json::to_vec(&content).map_err(|e| e.to_string())?)?;
    if errors.is_empty() {
        if let Some(authority) = authority {
            match authority.authorize_task(&request.context, &digest, request.action) {
                Ok(proof)
                    if proof.context == request.context
                        && proof.contract_digest == digest
                        && proof.action == request.action
                        && request
                            .context
                            .authorization_refs
                            .contains(&proof.authorization_ref) =>
                {
                    result.compiled = Some(CompiledContract {
                        digest,
                        context: request.context.clone(),
                        action: request.action,
                        layers: identities,
                        rules: result.rules.clone(),
                        obligations: result.obligations.clone(),
                        verifiers: pins,
                        authorization_ref: proof.authorization_ref,
                        schema_digest: schema.digest().clone(),
                        sources,
                        approvals,
                        authorization_authority: proof.authority_id,
                    });
                }
                Ok(_) => issue(
                    errors,
                    "authorization_binding_mismatch",
                    None,
                    None,
                    "existing authorization must bind exact compilation/context/action/reference",
                ),
                Err(_) => issue(
                    errors,
                    "authorization_rejected",
                    None,
                    None,
                    "independent authority rejected existing task authorization",
                ),
            }
        }
    }
    Ok(result)
}
