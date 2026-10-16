use guard_contract::{
    CapabilityProbe, CompilationStatus, CompileContext, CompileRequest, ContractLayer, LayerLevel,
    SourceMaterial, VerifierObservation, VerifierRegistry, compile, snapshot_digest,
};
use guard_protocol::{Action, Digest, Identifier, ProtocolDocument};
use serde_json::Value;
use sha2::{Digest as Hasher, Sha256};

struct Missing;
impl CapabilityProbe for Missing {
    fn observe(&self, _pin: &guard_contract::VerifierPin) -> VerifierObservation {
        VerifierObservation::Missing("tool not installed".into())
    }
}
fn id(s: &str) -> Identifier {
    Identifier::new(s).unwrap()
}
fn layer(level: LayerLevel) -> ContractLayer {
    let mut raw: Value = serde_json::from_slice(include_bytes!(
        "../../guard-protocol/fixtures/valid/contract-snapshot.json"
    ))
    .unwrap();
    let bytes = b"approved native architecture constraints".to_vec();
    raw["contractId"] = Value::String(format!("contract-{level:?}"));
    raw["sourceRefs"][0]["contentDigest"] =
        Value::String(format!("sha256:{:x}", Sha256::digest(&bytes)));
    let parsed = guard_protocol::decode(&serde_json::to_vec(&raw).unwrap()).unwrap();
    let ProtocolDocument::ContractSnapshot(mut snapshot) = parsed.document().clone() else {
        panic!("fixture")
    };
    snapshot.digest = snapshot_digest(&snapshot).unwrap();
    let source = snapshot.source_refs[0].clone();
    ContractLayer {
        level,
        document: serde_json::to_vec(&ProtocolDocument::ContractSnapshot(snapshot)).unwrap(),
        sources: vec![SourceMaterial { source, bytes }],
    }
}
fn request() -> CompileRequest {
    CompileRequest {
        layers: vec![
            layer(LayerLevel::Organization),
            layer(LayerLevel::Project),
            layer(LayerLevel::Task),
        ],
        context: CompileContext {
            repository_id: id("repo-example"),
            task_id: id("REQ-1"),
            authorization_refs: vec![id("AUTH-1")],
        },
        action: Action::Merge,
    }
}
#[test]
fn absent_authority_and_tools_block_without_erasing_rules() {
    let registry = VerifierRegistry::new(id("opa"));
    let result = compile(&request(), &registry, &Missing, None).unwrap();
    assert_eq!(result.status(), CompilationStatus::Blocked);
    assert_eq!(result.rules().len(), 1);
    assert!(!result.obligations().is_empty());
    assert!(result.compiled().is_none());
    for code in [
        "approval_authority_missing",
        "verifier_missing",
        "policy_engine_missing",
    ] {
        assert!(
            result.diagnostics().iter().any(|d| d.code == code),
            "missing {code}: {:?}",
            result.diagnostics()
        );
    }
}

use guard_contract::{
    ApprovalAttestation, ApprovalAuthority, TaskAuthorization, VerifierDescriptor, VerifierKind,
};
use std::collections::BTreeMap;
struct Approved {
    context: CompileContext,
    layers: BTreeMap<LayerLevel, (Digest, Identifier)>,
    expected_compilation: Option<Digest>,
}
impl Approved {
    fn for_request(request: &CompileRequest) -> Self {
        let layers = request
            .layers
            .iter()
            .map(|layer| {
                let decoded = guard_protocol::decode(&layer.document).unwrap();
                let ProtocolDocument::ContractSnapshot(s) = decoded.document() else {
                    panic!("snapshot")
                };
                (layer.level, (s.digest.clone(), s.approval_ref.clone()))
            })
            .collect();
        Self {
            context: request.context.clone(),
            layers,
            expected_compilation: None,
        }
    }
}
impl ApprovalAuthority for Approved {
    fn verify_layer(
        &self,
        context: &CompileContext,
        level: LayerLevel,
        approval_ref: &Identifier,
        digest: &Digest,
        action: Action,
    ) -> Result<ApprovalAttestation, String> {
        if context != &self.context
            || action != Action::Merge
            || self.layers.get(&level) != Some(&(digest.clone(), approval_ref.clone()))
        {
            return Err("not independently approved".into());
        }
        Ok(ApprovalAttestation {
            authority_id: id("test-control-authority"),
            approval_ref: approval_ref.clone(),
            contract_digest: digest.clone(),
            context: self.context.clone(),
            level,
            action,
        })
    }
    fn authorize_task(
        &self,
        context: &CompileContext,
        digest: &Digest,
        action: Action,
    ) -> Result<TaskAuthorization, String> {
        if context != &self.context
            || action != Action::Merge
            || self
                .expected_compilation
                .as_ref()
                .is_some_and(|d| d != digest)
        {
            return Err("authorization out of original scope".into());
        }
        Ok(TaskAuthorization {
            authority_id: id("test-control-authority"),
            authorization_ref: id("AUTH-1"),
            contract_digest: digest.clone(),
            context: self.context.clone(),
            action,
        })
    }
}
struct Present;
impl CapabilityProbe for Present {
    fn observe(&self, pin: &guard_contract::VerifierPin) -> VerifierObservation {
        VerifierObservation::Available(pin.clone())
    }
}
fn digest() -> Digest {
    Digest::new(format!("sha256:{}", "b".repeat(64))).unwrap()
}
fn descriptor(policy: bool) -> VerifierDescriptor {
    let schema = if policy {
        serde_json::json!({"type":"object","additionalProperties":false})
    } else {
        serde_json::json!({"$schema":"https://json-schema.org/draft/2020-12/schema","type":"object",
            "properties":{"maxCycles":{"type":"integer","minimum":0},"thresholdDecimal":{"type":"string","pattern":"^0\\.[0-9]+$"}},
            "required":["maxCycles","thresholdDecimal"],"additionalProperties":false})
    };
    VerifierDescriptor {
        verifier_id: id(if policy { "opa" } else { "java-package-check" }),
        version: id("1.0.0"),
        artifact_digest: digest(),
        config_digest: digest(),
        adapter_version: id("1.0.0"),
        kind: if policy {
            VerifierKind::PolicyEngine
        } else {
            VerifierKind::Domain
        },
        owner: if policy {
            None
        } else {
            Some(guard_protocol::DomainOwner::Archguard)
        },
        evidence_types: vec![id(if policy {
            "policy-decision"
        } else {
            "source-report"
        })],
        parameter_schema: serde_json::to_vec(&schema).unwrap(),
    }
}
fn registry() -> VerifierRegistry {
    let mut registry = VerifierRegistry::new(id("opa"));
    registry.register(descriptor(false)).unwrap();
    registry.register(descriptor(true)).unwrap();
    registry
}
fn update(layer: &mut ContractLayer, mut mutate: impl FnMut(&mut Value)) {
    let mut value: Value = serde_json::from_slice(&layer.document).unwrap();
    mutate(&mut value);
    let decoded = guard_protocol::decode(&serde_json::to_vec(&value).unwrap()).unwrap();
    let ProtocolDocument::ContractSnapshot(mut snapshot) = decoded.document().clone() else {
        panic!("fixture")
    };
    snapshot.digest = snapshot_digest(&snapshot).unwrap();
    layer.document = serde_json::to_vec(&ProtocolDocument::ContractSnapshot(snapshot)).unwrap();
}
fn has(result: &guard_contract::ContractCompilation, code: &str) -> bool {
    result.diagnostics().iter().any(|d| d.code == code)
}

#[test]
fn approved_sources_and_pinned_readonly_capabilities_produce_immutable_compile_result() {
    let request = request();
    let authority = Approved::for_request(&request);
    let registry = registry();
    let result = compile(&request, &registry, &Present, Some(&authority)).unwrap();
    assert_eq!(
        result.status(),
        CompilationStatus::ReadyForPlanning,
        "{:?}",
        result.diagnostics()
    );
    let contract = result.compiled().unwrap();
    assert_eq!(contract.rules().len(), 1);
    assert_eq!(contract.layers().len(), 3);
    assert_eq!(contract.verifiers().len(), 2);
    assert_eq!(contract.authorization_ref(), &id("AUTH-1"));
    assert_eq!(contract.context(), &request.context);
    assert_eq!(contract.action(), Action::Merge);
    let mut reordered = request.clone();
    reordered.layers.reverse();
    let repeated = compile(&reordered, &registry, &Present, Some(&authority)).unwrap();
    assert_eq!(contract.digest(), repeated.compiled().unwrap().digest());
}

#[test]
fn organization_rules_cannot_be_overridden_even_by_an_individually_approved_lower_layer() {
    let mut request = request();
    update(&mut request.layers[1], |v| {
        v["rules"][0]["mode"] = Value::String("ADVISE".into())
    });
    let authority = Approved::for_request(&request);
    let result = compile(&request, &registry(), &Present, Some(&authority)).unwrap();
    assert_eq!(result.status(), CompilationStatus::Blocked);
    assert!(has(&result, "organization_rule_locked"));
    assert_eq!(result.rules()[0].mode, guard_protocol::Mode::Enforce);
}

#[test]
fn same_project_rule_id_with_different_task_meaning_conflicts() {
    let mut request = request();
    for (i, layer) in request.layers.iter_mut().enumerate() {
        update(layer, |v| {
            let key = if i == 0 { "ORG-1" } else { "PROJECT-1" };
            v["rules"][0]["ruleId"] = Value::String(key.into());
            v["obligations"][0]["ruleId"] = Value::String(key.into());
            v["obligations"][0]["obligationId"] = Value::String(format!("OB-{key}"));
            if i == 2 {
                v["rules"][0]["parameters"]["maxCycles"] = serde_json::json!(1);
            }
        });
    }
    let authority = Approved::for_request(&request);
    let result = compile(&request, &registry(), &Present, Some(&authority)).unwrap();
    assert!(has(&result, "rule_definition_conflict"));
    assert_eq!(result.rules().len(), 2);
    assert!(result.compiled().is_none());
}

#[test]
fn candidate_cannot_reapprove_a_changed_snapshot_by_recomputing_its_own_digest() {
    let mut request = request();
    let authority = Approved::for_request(&request);
    update(&mut request.layers[2], |v| {
        v["version"] = Value::String("candidate-self-approved-revision".into())
    });
    let result = compile(&request, &registry(), &Present, Some(&authority)).unwrap();
    assert!(has(&result, "snapshot_not_approved"));
    assert!(result.compiled().is_none());
}

#[test]
fn actual_source_bytes_must_match_the_approved_reference() {
    let mut request = request();
    let authority = Approved::for_request(&request);
    request.layers[0].sources[0].bytes.push(b'!');
    let result = compile(&request, &registry(), &Present, Some(&authority)).unwrap();
    assert!(has(&result, "source_digest_mismatch"));
    assert_eq!(result.rules().len(), 1);
    assert!(result.compiled().is_none());
}

#[test]
fn missing_opa_missing_verifier_or_wrong_actual_pin_never_becomes_ready() {
    let request = request();
    let authority = Approved::for_request(&request);
    let mut no_opa = VerifierRegistry::new(id("opa"));
    no_opa.register(descriptor(false)).unwrap();
    let missing = compile(&request, &no_opa, &Present, Some(&authority)).unwrap();
    assert!(has(&missing, "policy_engine_missing"));
    assert_eq!(missing.rules().len(), 1);
    let unavailable = compile(&request, &registry(), &Missing, Some(&authority)).unwrap();
    assert!(has(&unavailable, "verifier_unavailable"));
    assert!(unavailable.compiled().is_none());
    struct Changed;
    impl CapabilityProbe for Changed {
        fn observe(&self, pin: &guard_contract::VerifierPin) -> VerifierObservation {
            let mut p = pin.clone();
            p.version = id("9.9.9");
            VerifierObservation::Available(p)
        }
    }
    let wrong = compile(&request, &registry(), &Changed, Some(&authority)).unwrap();
    assert!(has(&wrong, "verifier_identity_mismatch"));
    assert!(wrong.compiled().is_none());
}

#[test]
fn rule_parameters_are_evaluated_against_registered_schema() {
    let mut request = request();
    for layer in &mut request.layers {
        update(layer, |v| {
            v["rules"][0]["parameters"]["maxCycles"] = serde_json::json!(-1)
        });
    }
    let authority = Approved::for_request(&request);
    let result = compile(&request, &registry(), &Present, Some(&authority)).unwrap();
    assert!(has(&result, "verifier_parameters_invalid"));
    assert!(
        result
            .diagnostics()
            .iter()
            .any(|d| d.detail.contains("/maxCycles"))
    );
    assert!(result.compiled().is_none());
}

#[test]
fn registry_rejects_mutable_versions_conflicting_identities_and_external_schema_refs() {
    let mut registry = registry();
    let original = registry.pin(&id("java-package-check")).unwrap().clone();
    let mut changed = descriptor(false);
    changed.version = id("2.0.0");
    assert!(registry.register(changed).is_err());
    assert_eq!(registry.pin(&id("java-package-check")).unwrap(), &original);
    let mut empty = VerifierRegistry::new(id("opa"));
    for version in ["latest", "1.x", "^1.0.0", ">=1.0.0"] {
        let mut d = descriptor(false);
        d.version = id(version);
        assert!(empty.register(d).is_err());
    }
    let mut external = descriptor(false);
    external.parameter_schema = br#"{"$ref":"file:///E:/forbidden.json"}"#.to_vec();
    assert!(empty.register(external).is_err());
    assert!(empty.pin(&id("java-package-check")).is_none());
}

#[test]
fn a_new_verifier_pin_requires_authorization_for_the_new_compilation_identity() {
    let request = request();
    let mut authority = Approved::for_request(&request);
    let old = compile(&request, &registry(), &Present, Some(&authority)).unwrap();
    authority.expected_compilation = Some(old.compiled().unwrap().digest().clone());
    let mut updated = VerifierRegistry::new(id("opa"));
    let mut checker = descriptor(false);
    checker.version = id("2.0.0");
    updated.register(checker).unwrap();
    updated.register(descriptor(true)).unwrap();
    let result = compile(&request, &updated, &Present, Some(&authority)).unwrap();
    assert!(has(&result, "authorization_rejected"));
    assert!(result.compiled().is_none());
}

#[test]
fn approved_duplicate_obligation_ids_cannot_change_target_sets() {
    let mut request = request();
    update(&mut request.layers[2], |v| {
        v["obligations"][0]["targets"][0]["protectedDigest"] =
            Value::String(format!("sha256:{}", "c".repeat(64)))
    });
    let authority = Approved::for_request(&request);
    let result = compile(&request, &registry(), &Present, Some(&authority)).unwrap();
    assert!(has(&result, "obligation_definition_conflict"));
    assert_eq!(result.obligations().len(), 1);
    assert!(result.compiled().is_none());
}

#[test]
fn trusted_text_in_candidate_json_and_duplicate_keys_do_not_replace_authority() {
    let mut request = request();
    let authority = Approved::for_request(&request);
    let mut raw: Value = serde_json::from_slice(&request.layers[0].document).unwrap();
    raw["trusted"] = serde_json::json!(true);
    request.layers[0].document = serde_json::to_vec(&raw).unwrap();
    let candidate = compile(&request, &registry(), &Present, Some(&authority)).unwrap();
    assert!(has(&candidate, "protocol_schema_invalid"));
    assert!(candidate.compiled().is_none());
    let mut request = super_request();
    let mut duplicate = b"{\"schemaVersion\":\"1.0.0\",".to_vec();
    duplicate.extend_from_slice(&request.layers[0].document[1..]);
    request.layers[0].document = duplicate;
    let duplicate = compile(&request, &registry(), &Present, Some(&authority)).unwrap();
    assert!(has(&duplicate, "protocol_json_invalid"));
}
fn super_request() -> CompileRequest {
    request()
}

#[test]
fn independently_returned_approval_and_authorization_must_match_the_exact_binding() {
    struct Misbound(Approved);
    impl ApprovalAuthority for Misbound {
        fn verify_layer(
            &self,
            c: &CompileContext,
            l: LayerLevel,
            r: &Identifier,
            d: &Digest,
            a: Action,
        ) -> Result<ApprovalAttestation, String> {
            let mut proof = self.0.verify_layer(c, l, r, d, a)?;
            proof.context.task_id = id("FOREIGN-TASK");
            Ok(proof)
        }
        fn authorize_task(
            &self,
            c: &CompileContext,
            d: &Digest,
            a: Action,
        ) -> Result<TaskAuthorization, String> {
            self.0.authorize_task(c, d, a)
        }
    }
    let request = request();
    let authority = Misbound(Approved::for_request(&request));
    let result = compile(&request, &registry(), &Present, Some(&authority)).unwrap();
    assert!(has(&result, "approval_binding_mismatch"));
    assert!(result.compiled().is_none());
    struct ForeignAuthorization(Approved);
    impl ApprovalAuthority for ForeignAuthorization {
        fn verify_layer(
            &self,
            c: &CompileContext,
            l: LayerLevel,
            r: &Identifier,
            d: &Digest,
            a: Action,
        ) -> Result<ApprovalAttestation, String> {
            self.0.verify_layer(c, l, r, d, a)
        }
        fn authorize_task(
            &self,
            c: &CompileContext,
            d: &Digest,
            a: Action,
        ) -> Result<TaskAuthorization, String> {
            let mut proof = self.0.authorize_task(c, d, a)?;
            proof.authorization_ref = id("NOT-IN-USER-SCOPE");
            Ok(proof)
        }
    }
    let authority = ForeignAuthorization(Approved::for_request(&request));
    let result = compile(&request, &registry(), &Present, Some(&authority)).unwrap();
    assert!(has(&result, "authorization_binding_mismatch"));
    assert!(result.compiled().is_none());
}

#[test]
fn registered_owner_and_evidence_cannot_impersonate_a_different_professional_guard() {
    let request = request();
    let authority = Approved::for_request(&request);
    let mut registry = VerifierRegistry::new(id("opa"));
    let mut checker = descriptor(false);
    checker.owner = Some(guard_protocol::DomainOwner::Codeguard);
    checker.evidence_types = vec![id("different-report")];
    registry.register(checker).unwrap();
    registry.register(descriptor(true)).unwrap();
    let result = compile(&request, &registry, &Present, Some(&authority)).unwrap();
    assert!(has(&result, "verifier_owner_mismatch"));
    assert!(has(&result, "verifier_evidence_unsupported"));
    assert!(result.compiled().is_none());
}

#[test]
fn missing_source_and_missing_registered_verifier_are_diagnosed_with_approval_present() {
    let mut request = request();
    let authority = Approved::for_request(&request);
    request.layers[0].sources.clear();
    let result = compile(&request, &registry(), &Present, Some(&authority)).unwrap();
    assert!(has(&result, "source_missing_or_duplicate"));
    let mut request = super_request();
    for layer in &mut request.layers {
        update(layer, |v| {
            v["rules"][0]["verifierId"] = Value::String("missing-checker".into());
            v["obligations"][0]["verifierId"] = Value::String("missing-checker".into());
        });
    }
    let authority = Approved::for_request(&request);
    let result = compile(&request, &registry(), &Present, Some(&authority)).unwrap();
    assert!(has(&result, "verifier_missing"));
    assert_eq!(result.rules().len(), 1);
    assert!(result.compiled().is_none());
}

#[test]
fn approved_selection_does_not_require_a_candidate_source_digest() {
    let mut request = request();
    for layer in &mut request.layers {
        update(layer, |v| {
            let target = v["obligations"][0]["targets"][0].as_object_mut().unwrap();
            target.remove("inputDigest");
            target.remove("protectedDigest");
        });
    }
    let authority = Approved::for_request(&request);
    let result = compile(&request, &registry(), &Present, Some(&authority)).unwrap();
    let compiled = result
        .compiled()
        .expect("approved selection is ready for candidate binding");
    let serialized = serde_json::to_value(&compiled.obligations()[0]).unwrap();
    let target = serialized["targets"][0].as_object().unwrap();
    assert!(target.contains_key("path"));
    assert!(!target.contains_key("inputDigest"));
    assert!(!target.contains_key("protectedDigest"));
    let selection = &compiled.obligations()[0].targets[0];
    let first_candidate = Digest::new(format!("sha256:{}", "d".repeat(64))).unwrap();
    let next_candidate = Digest::new(format!("sha256:{}", "e".repeat(64))).unwrap();
    let first_binding = selection.bind(first_candidate).unwrap();
    let next_binding = selection.bind(next_candidate).unwrap();
    assert_ne!(first_binding.input_digest, next_binding.input_digest);
    assert_eq!(first_binding.path, next_binding.path);
    let repeated = compile(&request, &registry(), &Present, Some(&authority)).unwrap();
    assert_eq!(compiled.digest(), repeated.compiled().unwrap().digest());
}

#[test]
fn protected_digest_is_retained_and_cannot_be_reapproved_by_the_candidate() {
    let mut request = request();
    let protected = format!("sha256:{}", "b".repeat(64));
    for layer in &mut request.layers {
        update(layer, |v| {
            let target = v["obligations"][0]["targets"][0].as_object_mut().unwrap();
            target.remove("inputDigest");
            target.insert("protectedDigest".into(), Value::String(protected.clone()));
        });
    }
    let authority = Approved::for_request(&request);
    let original = compile(&request, &registry(), &Present, Some(&authority)).unwrap();
    let compiled = original.compiled().expect("approved protected selection");
    assert_eq!(
        serde_json::to_value(&compiled.obligations()[0]).unwrap()["targets"][0]["protectedDigest"],
        protected
    );
    for layer in &mut request.layers {
        update(layer, |v| {
            v["obligations"][0]["targets"][0]["protectedDigest"] =
                Value::String(format!("sha256:{}", "c".repeat(64)));
        });
    }
    let revised = compile(&request, &registry(), &Present, Some(&authority)).unwrap();
    assert!(has(&revised, "snapshot_not_approved"));
    assert!(revised.compiled().is_none());
}
