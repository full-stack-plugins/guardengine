use base64::{Engine, engine::general_purpose::STANDARD};
use guard_authority::{
    AnchorSource, ApprovalMaterial, ApprovalPermission, ApprovalPurpose, ApprovalVerifier,
    ApprovedScope, AuthorityAnchor, AuthorityClock, PrincipalRole, ReceiptBundle, ReceiptSource,
    RevocationSnapshot, RevocationSource, VerificationRequest, signing_bytes,
};
use guard_protocol::{
    Action, ApprovalIntent, Digest, GitPath, Identifier, ProtocolDocument, TargetSelection,
    UtcTimestamp,
};
use ring::signature::{Ed25519KeyPair, KeyPair};
use std::sync::{Arc, Mutex};
fn id(s: &str) -> Identifier {
    Identifier::new(s).unwrap()
}
fn time(s: &str) -> UtcTimestamp {
    UtcTimestamp::new(s).unwrap()
}
fn digest(s: &[u8]) -> Digest {
    guard_protocol::digest_json(s).unwrap()
}
struct Ports {
    anchor: Mutex<AuthorityAnchor>,
    bundle: Mutex<ReceiptBundle>,
    revoked: Mutex<bool>,
    revision: Mutex<u64>,
    now: Mutex<UtcTimestamp>,
    unavailable: Mutex<bool>,
    advance_during_revocation: Mutex<bool>,
    wrong_revocation_identity: Mutex<bool>,
}
impl AnchorSource for Ports {
    fn current(&self, key: &Identifier) -> Result<AuthorityAnchor, String> {
        let a = self.anchor.lock().unwrap().clone();
        if &a.key_id != key {
            return Err("unknown key".into());
        }
        Ok(a)
    }
}
impl ReceiptSource for Ports {
    fn get(&self, _reference: &Identifier) -> Result<ReceiptBundle, String> {
        Ok(self.bundle.lock().unwrap().clone())
    }
}
impl AuthorityClock for Ports {
    fn now(&self) -> Result<UtcTimestamp, String> {
        Ok(self.now.lock().unwrap().clone())
    }
}
impl RevocationSource for Ports {
    fn current(
        &self,
        issuer: &Identifier,
        key: &Identifier,
        receipt_id: &Identifier,
        reference: &Identifier,
    ) -> Result<RevocationSnapshot, String> {
        if *self.unavailable.lock().unwrap() {
            return Err("independent revocation unavailable".into());
        }
        let revision = *self.revision.lock().unwrap();
        if *self.advance_during_revocation.lock().unwrap() {
            *self.now.lock().unwrap() = time("2026-10-17T00:06:00Z");
        }
        Ok(RevocationSnapshot {
            authority: id("protected-revocation"),
            issuer: issuer.clone(),
            key_id: key.clone(),
            receipt_id: if *self.wrong_revocation_identity.lock().unwrap() {
                id("wrong-receipt")
            } else {
                receipt_id.clone()
            },
            revocation_ref: reference.clone(),
            state_digest: digest(format!("{{\"revision\":{}}}", revision).as_bytes()),
            revision,
            valid_from: time("2026-10-17T00:00:00Z"),
            expires_at: time("2026-10-17T00:10:00Z"),
            revoked: *self.revoked.lock().unwrap(),
        })
    }
}
fn scope() -> ApprovedScope {
    ApprovedScope {
        repository_id: id("repo-example"),
        task_id: id("REQ-1"),
        actions: vec![Action::Merge],
        targets: vec![TargetSelection {
            repository_id: id("repo-example"),
            path: GitPath::from_bytes(b"src/Example.java").unwrap(),
            selector: None,
            protected_digest: None,
        }],
    }
}
fn material() -> ApprovalMaterial {
    ApprovalMaterial {
        schema_version: "guard.approval-material/v1".into(),
        audience: id("controller-1"),
        requested_by: id("coding-agent"),
        action: Action::Merge,
        purpose: ApprovalPurpose::Task,
        scope: scope(),
    }
}
fn signed(pair: &Ed25519KeyPair, material: &ApprovalMaterial) -> ReceiptBundle {
    let document = guard_protocol::decode(include_bytes!(
        "../../guard-protocol/fixtures/valid/approval-receipt.json"
    ))
    .unwrap();
    let ProtocolDocument::ApprovalReceipt(mut receipt) = document.document().clone() else {
        panic!("fixture")
    };
    receipt.receipt_id = id("APPROVAL-1");
    receipt.issuer = id("external-issuer");
    receipt.principal = id("reviewer-1");
    receipt.role = id("architect");
    receipt.intent = ApprovalIntent::RequirementScope;
    receipt.scope_digest = material.scope.digest().unwrap();
    receipt.contract_digest = digest(br#"{"contract":1}"#);
    receipt.subject_binding.kind = receipt.intent;
    receipt.subject_binding.subject_digest = None;
    receipt.subject_binding.scope_digest = receipt.scope_digest.clone();
    let bytes = serde_json::to_vec(material).unwrap();
    receipt.proof.provenance_digest = digest(&bytes);
    receipt.proof.key_id = id("external-key-1");
    receipt.proof.signature =
        id(&STANDARD.encode(pair.sign(&signing_bytes(&receipt).unwrap()).as_ref()));
    ReceiptBundle {
        receipt: serde_json::to_vec(&ProtocolDocument::ApprovalReceipt(receipt)).unwrap(),
        material: bytes,
    }
}
fn fixture() -> (Arc<Ports>, VerificationRequest, ApprovalVerifier) {
    let pair = Ed25519KeyPair::from_seed_unchecked(&[37; 32]).unwrap();
    let material = material();
    let bundle = signed(&pair, &material);
    let request = VerificationRequest {
        reference: id("APPROVAL-1"),
        material,
        contract_digest: digest(br#"{"contract":1}"#),
        intent: ApprovalIntent::RequirementScope,
        subject_digest: None,
    };
    let ports = Arc::new(Ports {
        anchor: Mutex::new(AuthorityAnchor {
            key_id: id("external-key-1"),
            revocation_authority: id("protected-revocation"),
            issuer: id("external-issuer"),
            public_key: pair.public_key().as_ref().try_into().unwrap(),
            identities: vec![PrincipalRole {
                principal: id("reviewer-1"),
                role: id("architect"),
                purposes: vec![
                    ApprovalPermission::OrganizationLayer,
                    ApprovalPermission::ProjectLayer,
                    ApprovalPermission::TaskLayer,
                    ApprovalPermission::Task,
                    ApprovalPermission::Migration,
                ],
                intents: vec![
                    ApprovalIntent::RequirementScope,
                    ApprovalIntent::CandidateAcceptance,
                    ApprovalIntent::OperationAuthorization,
                ],
            }],
            audiences: vec![id("controller-1")],
            intents: vec![
                ApprovalIntent::RequirementScope,
                ApprovalIntent::CandidateAcceptance,
                ApprovalIntent::OperationAuthorization,
            ],
        }),
        bundle: Mutex::new(bundle),
        revoked: Mutex::new(false),
        revision: Mutex::new(1),
        now: Mutex::new(time("2026-10-17T00:01:00Z")),
        unavailable: Mutex::new(false),
        advance_during_revocation: Mutex::new(false),
        wrong_revocation_identity: Mutex::new(false),
    });
    let verifier =
        ApprovalVerifier::new(ports.clone(), ports.clone(), ports.clone(), ports.clone());
    (ports, request, verifier)
}
#[test]
fn real_ed25519_receipt_requires_independent_exact_binding() {
    let (ports, request, verifier) = fixture();
    let verified = verifier.verify(&request).unwrap();
    assert_eq!(verified.reference(), &request.reference);
    assert_eq!(verified.contract_digest(), &request.contract_digest);
    {
        let mut anchor = ports.anchor.lock().unwrap();
        let mut identity = anchor.identities[0].clone();
        identity.principal = id("reviewer-2");
        anchor.identities.push(identity);
    }
    let original = ports.bundle.lock().unwrap().clone();
    rewrite(&ports, |receipt, _| receipt.principal = id("reviewer-2"));
    // 独立新实例证明替换原件确实有效；已观察旧ID的实例必须拒绝替换。
    ApprovalVerifier::new(ports.clone(), ports.clone(), ports.clone(), ports.clone())
        .verify(&request)
        .unwrap();
    assert!(verifier.verify(&request).is_err());
    *ports.bundle.lock().unwrap() = original;
    assert!(
        verifier.verify(&request).is_err(),
        "restoring original bytes must not erase observed identity conflict"
    );
    assert!(
        verifier.revalidate(&verified).is_err(),
        "revalidation cannot silently substitute a different independently permitted approver"
    );
}

fn rewrite(
    ports: &Ports,
    mutator: impl FnOnce(&mut guard_protocol::ApprovalReceipt, &mut ApprovalMaterial),
) {
    let mut bundle = ports.bundle.lock().unwrap();
    let decoded = guard_protocol::decode(&bundle.receipt).unwrap();
    let ProtocolDocument::ApprovalReceipt(mut receipt) = decoded.document().clone() else {
        panic!("fixture")
    };
    let mut material = ApprovalMaterial::parse(&bundle.material).unwrap();
    mutator(&mut receipt, &mut material);
    bundle.material = serde_json::to_vec(&material).unwrap();
    receipt.proof.provenance_digest = digest(&bundle.material);
    let pair = Ed25519KeyPair::from_seed_unchecked(&[37; 32]).unwrap();
    receipt.proof.signature =
        id(&STANDARD.encode(pair.sign(&signing_bytes(&receipt).unwrap()).as_ref()));
    bundle.receipt = serde_json::to_vec(&ProtocolDocument::ApprovalReceipt(receipt)).unwrap();
}
#[test]
fn signed_actor_role_audience_scope_and_contract_must_match_independent_authority() {
    for case in 0..7 {
        let (ports, request, verifier) = fixture();
        rewrite(&ports, |receipt, material| match case {
            0 => receipt.principal = id("fake-user"),
            1 => receipt.role = id("admin"),
            2 => material.audience = id("other-controller"),
            3 => material.scope.targets.push(TargetSelection {
                repository_id: id("repo-example"),
                path: GitPath::from_bytes(b"secrets.java").unwrap(),
                selector: None,
                protected_digest: None,
            }),
            4 => receipt.contract_digest = digest(b"{\"different\":true}"),
            5 => receipt.issuer = id("other-issuer"),
            _ => material.requested_by = receipt.principal.clone(),
        });
        assert!(verifier.verify(&request).is_err(), "case {case}");
    }
    let (ports, request, verifier) = fixture();
    ports.anchor.lock().unwrap().identities[0].purposes = vec![ApprovalPermission::TaskLayer];
    assert!(
        verifier.verify(&request).is_err(),
        "role is not allowed task authorization"
    );
}
#[test]
fn malformed_unknown_duplicate_and_wrong_signature_domains_fail_closed() {
    for case in 0..6 {
        let (ports, request, verifier) = fixture();
        let mut bundle = ports.bundle.lock().unwrap();
        let mut value: serde_json::Value = serde_json::from_slice(&bundle.receipt).unwrap();
        match case {
            0 => {
                value["principal"] = serde_json::json!("tampered");
                bundle.receipt = serde_json::to_vec(&value).unwrap();
            }
            1 => {
                value["proof"]["signature"] = serde_json::json!("0".repeat(128));
                bundle.receipt = serde_json::to_vec(&value).unwrap();
            }
            2 => {
                bundle.receipt = String::from_utf8(bundle.receipt.clone())
                    .unwrap()
                    .replacen("{", "{\"receiptId\":\"DUP\",", 1)
                    .into_bytes();
            }
            3 => {
                value["schemaVersion"] = serde_json::json!("2.0.0");
                bundle.receipt = serde_json::to_vec(&value).unwrap();
            }
            4 => {
                let pair = Ed25519KeyPair::from_seed_unchecked(&[37; 32]).unwrap();
                value["proof"]["signature"] = serde_json::json!(
                    STANDARD.encode(
                        pair.sign(b"guard.signed-evidence/v1\nnot-new-receipt")
                            .as_ref()
                    )
                );
                bundle.receipt = serde_json::to_vec(&value).unwrap();
            }
            _ => {
                let mut material: serde_json::Value =
                    serde_json::from_slice(&bundle.material).unwrap();
                material["trusted"] = serde_json::json!(true);
                bundle.material = serde_json::to_vec(&material).unwrap();
            }
        }
        drop(bundle);
        assert!(verifier.verify(&request).is_err(), "case {case}");
    }
}
#[test]
fn weak_anchor_and_malleable_signature_are_rejected() {
    let (ports, request, verifier) = fixture();
    let mut weak = [0u8; 32];
    weak[0] = 1;
    ports.anchor.lock().unwrap().public_key = weak;
    let mut value: serde_json::Value =
        serde_json::from_slice(&ports.bundle.lock().unwrap().receipt).unwrap();
    let mut signature = [0u8; 64];
    signature[0] = 1;
    value["proof"]["signature"] = serde_json::json!(STANDARD.encode(signature));
    ports.bundle.lock().unwrap().receipt = serde_json::to_vec(&value).unwrap();
    assert!(verifier.verify(&request).unwrap_err().contains("weak"));
    let (ports, request, verifier) = fixture();
    let mut value: serde_json::Value =
        serde_json::from_slice(&ports.bundle.lock().unwrap().receipt).unwrap();
    value["proof"]["signature"] = serde_json::json!(STANDARD.encode([255u8; 64]));
    ports.bundle.lock().unwrap().receipt = serde_json::to_vec(&value).unwrap();
    assert!(verifier.verify(&request).is_err());
}
#[test]
fn current_time_revocation_unavailable_rollback_and_same_revision_equivocation_reject() {
    for instant in ["2026-10-16T23:59:59Z", "2026-10-17T00:05:00Z"] {
        let (ports, request, verifier) = fixture();
        *ports.now.lock().unwrap() = time(instant);
        assert!(verifier.verify(&request).is_err());
    }
    let (ports, request, verifier) = fixture();
    verifier.verify(&request).unwrap();
    *ports.unavailable.lock().unwrap() = true;
    assert!(verifier.verify(&request).is_err());
    *ports.unavailable.lock().unwrap() = false;
    *ports.revision.lock().unwrap() = 2;
    *ports.revoked.lock().unwrap() = true;
    assert!(verifier.verify(&request).is_err());
    *ports.revoked.lock().unwrap() = false;
    assert!(
        verifier
            .verify(&request)
            .unwrap_err()
            .contains("equivocation")
    );
    *ports.revision.lock().unwrap() = 1;
    assert!(verifier.verify(&request).unwrap_err().contains("rollback"));
    let (ports, request, verifier) = fixture();
    *ports.now.lock().unwrap() = time("2026-10-17T00:01:00.10Z");
    verifier.verify(&request).unwrap();
    *ports.now.lock().unwrap() = time("2026-10-17T00:01:00.1Z");
    verifier.verify(&request).unwrap();
    *ports.now.lock().unwrap() = time("2026-10-17T00:01:00.09Z");
    assert!(verifier.verify(&request).is_err());
}
#[test]
fn candidate_approval_is_exact_and_scope_receipt_cannot_change_kind() {
    let (ports, mut request, verifier) = fixture();
    let subject = digest(b"{\"candidate\":1}");
    rewrite(&ports, |receipt, _| {
        receipt.intent = ApprovalIntent::CandidateAcceptance;
        receipt.subject_binding.kind = receipt.intent;
        receipt.subject_binding.subject_digest = Some(subject.clone());
    });
    assert!(verifier.verify(&request).is_err());
    request.intent = ApprovalIntent::CandidateAcceptance;
    request.subject_digest = Some(subject);
    verifier.verify(&request).unwrap();
    request.subject_digest = Some(digest(b"{\"candidate\":2}"));
    assert!(verifier.verify(&request).is_err());
}

#[test]
fn signature_payload_is_independently_assembled_and_includes_complete_proof_metadata() {
    let (ports, request, verifier) = fixture();
    let bundle = ports.bundle.lock().unwrap().clone();
    let mut raw: serde_json::Value = serde_json::from_slice(&bundle.receipt).unwrap();
    let declared = raw["proof"]["signature"].as_str().unwrap().to_string();
    raw["proof"].as_object_mut().unwrap().remove("signature");
    // 独立组装：固定literal域+原wire删唯一signature；不调用生产signing_bytes计算expected。
    let mut message = b"guard.approval-receipt/v1\n".to_vec();
    message.extend(guard_protocol::canonicalize(&serde_json::to_vec(&raw).unwrap()).unwrap());
    let pair = Ed25519KeyPair::from_seed_unchecked(&[37; 32]).unwrap();
    assert_eq!(declared, STANDARD.encode(pair.sign(&message).as_ref()));
    let document = guard_protocol::decode(&bundle.receipt).unwrap();
    let ProtocolDocument::ApprovalReceipt(receipt) = document.document() else {
        panic!("fixture")
    };
    assert_eq!(message, signing_bytes(receipt).unwrap());
    for path in ["algorithm", "keyId", "provenanceDigest"] {
        let mut changed = raw.clone();
        changed["proof"][path] = serde_json::json!("changed");
        let mut bytes = b"guard.approval-receipt/v1\n".to_vec();
        bytes.extend(guard_protocol::canonicalize(&serde_json::to_vec(&changed).unwrap()).unwrap());
        assert_ne!(STANDARD.encode(pair.sign(&bytes).as_ref()), declared);
    }
    let verified = verifier.verify(&request).unwrap();
    *ports.revoked.lock().unwrap() = true;
    *ports.revision.lock().unwrap() = 2;
    assert!(verifier.revalidate(&verified).is_err());
}

#[test]
fn expiry_during_slow_revocation_lookup_is_rechecked_before_return() {
    let (ports, request, verifier) = fixture();
    *ports.advance_during_revocation.lock().unwrap() = true;
    assert!(
        verifier.verify(&request).is_err(),
        "receipt expired while the independent revocation source answered"
    );
}

#[test]
fn exact_scope_is_order_independent_and_keeps_protected_digest_and_raw_paths() {
    let mut first = scope();
    first.actions.push(Action::Commit);
    first.targets.push(TargetSelection {
        repository_id: id("repo-example"),
        path: GitPath::from_bytes(b"src/Other.java").unwrap(),
        selector: None,
        protected_digest: Some(digest(b"{\"protected\":1}")),
    });
    let mut reversed = first.clone();
    reversed.actions.reverse();
    reversed.targets.reverse();
    assert_eq!(first.digest().unwrap(), reversed.digest().unwrap());
    reversed.targets[0].protected_digest = None;
    assert_ne!(first.digest().unwrap(), reversed.digest().unwrap());
    let mut duplicate = first.clone();
    duplicate.targets.push(duplicate.targets[0].clone());
    assert!(duplicate.digest().is_err());
    let mut pattern = first.clone();
    pattern.targets[0].selector = Some(id("method:*"));
    assert!(pattern.digest().is_err());
    let mut changed = first.clone();
    changed.targets[0].path = GitPath::from_bytes(b"SRC/Example.java").unwrap();
    assert_ne!(first.digest().unwrap(), changed.digest().unwrap());
}

#[test]
fn adding_unsigned_explicit_null_optional_field_cannot_preserve_signature() {
    let (ports, request, verifier) = fixture();
    let mut raw: serde_json::Value =
        serde_json::from_slice(&ports.bundle.lock().unwrap().receipt).unwrap();
    raw["subjectBinding"]["subjectDigest"] = serde_json::Value::Null;
    ports.bundle.lock().unwrap().receipt = serde_json::to_vec(&raw).unwrap();
    assert!(
        verifier.verify(&request).is_err(),
        "typed reserialization must not silently drop an unsigned explicit-null wire field"
    );
}

#[test]
fn observed_revocation_cannot_be_reversed_by_a_higher_active_snapshot() {
    let (ports, request, verifier) = fixture();
    verifier.verify(&request).unwrap();
    *ports.revoked.lock().unwrap() = true;
    *ports.revision.lock().unwrap() = 2;
    assert!(verifier.verify(&request).is_err());
    *ports.revoked.lock().unwrap() = false;
    *ports.revision.lock().unwrap() = 3;
    assert!(
        verifier.verify(&request).is_err(),
        "an old receipt cannot revive after observed revocation; a new receipt/key is required"
    );
    rewrite(&ports, |receipt, _| {
        receipt.revocation_ref = id("other-list")
    });
    *ports.revision.lock().unwrap() = 4;
    assert!(
        verifier.verify(&request).is_err(),
        "changing list reference cannot undo observed receipt revocation"
    );
    rewrite(&ports, |receipt, _| receipt.receipt_id = id("APPROVAL-2"));
    let mut fresh = request.clone();
    fresh.reference = id("APPROVAL-2");
    verifier
        .verify(&fresh)
        .expect("a separately signed new receipt can be independently approved");
}

#[test]
fn mismatched_revocation_snapshot_identity_is_not_current_authority() {
    let (ports, request, verifier) = fixture();
    *ports.wrong_revocation_identity.lock().unwrap() = true;
    assert!(
        verifier.verify(&request).is_err(),
        "snapshot must bind issuer/key/receipt/ref tuple"
    );
}
#[test]
fn distinct_receipts_can_share_one_current_revocation_list_revision() {
    let (ports, mut request, verifier) = fixture();
    verifier.verify(&request).unwrap();
    rewrite(&ports, |receipt, _| receipt.receipt_id = id("APPROVAL-2"));
    request.reference = id("APPROVAL-2");
    verifier
        .verify(&request)
        .expect("separate receipt entries sharing a list must not collide");
}

#[test]
fn rotating_to_a_new_trusted_key_cannot_revive_a_revoked_receipt_id() {
    let (ports, request, verifier) = fixture();
    let previous = verifier.verify(&request).unwrap();
    *ports.revoked.lock().unwrap() = true;
    *ports.revision.lock().unwrap() = 2;
    assert!(verifier.verify(&request).is_err());
    *ports.revoked.lock().unwrap() = false;
    *ports.revision.lock().unwrap() = 3;
    let pair = Ed25519KeyPair::from_seed_unchecked(&[53; 32]).unwrap();
    {
        let mut anchor = ports.anchor.lock().unwrap();
        anchor.key_id = id("external-key-2");
        anchor.public_key = pair.public_key().as_ref().try_into().unwrap();
    }
    let mut bundle = ports.bundle.lock().unwrap();
    let decoded = guard_protocol::decode(&bundle.receipt).unwrap();
    let ProtocolDocument::ApprovalReceipt(mut receipt) = decoded.document().clone() else {
        panic!("fixture")
    };
    receipt.proof.key_id = id("external-key-2");
    receipt.proof.signature =
        id(&STANDARD.encode(pair.sign(&signing_bytes(&receipt).unwrap()).as_ref()));
    bundle.receipt =
        serde_json::to_vec(&ProtocolDocument::ApprovalReceipt(receipt.clone())).unwrap();
    drop(bundle);
    assert!(
        verifier.verify(&request).is_err(),
        "new trusted key cannot revive old issuer/receipt logical identity"
    );
    assert!(verifier.revalidate(&previous).is_err());
    receipt.receipt_id = id("APPROVAL-2");
    receipt.proof.signature =
        id(&STANDARD.encode(pair.sign(&signing_bytes(&receipt).unwrap()).as_ref()));
    ports.bundle.lock().unwrap().receipt =
        serde_json::to_vec(&ProtocolDocument::ApprovalReceipt(receipt)).unwrap();
    let mut next = request;
    next.reference = id("APPROVAL-2");
    verifier
        .verify(&next)
        .expect("new receipt identity may be separately approved by rotated trusted key");
}

#[test]
fn operation_authorization_requires_exact_subject_and_action() {
    let (ports, mut request, verifier) = fixture();
    let subject = digest(b"{\"operationCandidate\":1}");
    rewrite(&ports, |receipt, _| {
        receipt.intent = ApprovalIntent::OperationAuthorization;
        receipt.subject_binding.kind = receipt.intent;
        receipt.subject_binding.subject_digest = Some(subject.clone());
    });
    request.intent = ApprovalIntent::OperationAuthorization;
    request.subject_digest = Some(subject.clone());
    verifier
        .verify(&request)
        .expect("real signed exact operation authorization");
    request.subject_digest = Some(digest(b"{\"operationCandidate\":2}"));
    assert!(
        verifier.verify(&request).is_err(),
        "operation authorization cannot move to another candidate"
    );
    request.subject_digest = Some(subject);
    request.material.action = Action::Push;
    assert!(
        verifier.verify(&request).is_err(),
        "operation authorization cannot move to another action"
    );
}
