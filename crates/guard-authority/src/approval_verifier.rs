use crate::freshness_state::FreshnessState;
use crate::{
    AnchorSource, ApprovalMaterial, AuthorityClock, ReceiptSource, RevocationSource,
    VerificationRequest, VerifiedApproval, wire_signing_bytes,
};
use base64::{Engine, engine::general_purpose::STANDARD};
use ed25519_dalek::{Signature, VerifyingKey};
use guard_protocol::{ApprovalIntent, ProtocolDocument};
use std::{
    cmp::Ordering,
    sync::{Arc, Mutex},
};
/// 对受保护端口返回的原件执行独立签名与身份校验；不签grant。无Java对应类型。
pub struct ApprovalVerifier {
    anchors: Arc<dyn AnchorSource>,
    receipts: Arc<dyn ReceiptSource>,
    revocations: Arc<dyn RevocationSource>,
    clock: Arc<dyn AuthorityClock>,
    freshness: Mutex<FreshnessState>,
}
impl ApprovalVerifier {
    /// 注入受保护端口；其部署隔离须由宿主另证，不读取候选配置。
    pub fn new(
        anchors: Arc<dyn AnchorSource>,
        receipts: Arc<dyn ReceiptSource>,
        revocations: Arc<dyn RevocationSource>,
        clock: Arc<dyn AuthorityClock>,
    ) -> Self {
        Self {
            anchors,
            receipts,
            revocations,
            clock,
            freshness: Mutex::new(FreshnessState::default()),
        }
    }
    /// 在实际消费边界重新读取全部端口；旧对象clone不能绕过撤销和期限。
    pub fn revalidate(&self, previous: &VerifiedApproval) -> Result<VerifiedApproval, String> {
        let current = self.verify(&VerificationRequest {
            reference: previous.receipt.receipt_id.clone(),
            material: previous.material.clone(),
            contract_digest: previous.receipt.contract_digest.clone(),
            intent: previous.receipt.intent,
            subject_digest: previous.receipt.subject_binding.subject_digest.clone(),
        })?;
        if current.receipt != previous.receipt {
            return Err("approval receipt identity/content changed during revalidation".into());
        }
        Ok(current)
    }
    /// 核验当前来源和精确请求；缺任一证明拒绝，不产生默认批准。
    pub fn verify(&self, request: &VerificationRequest) -> Result<VerifiedApproval, String> {
        request.material.scope.digest()?;
        let bundle = self.receipts.get(&request.reference)?;
        if bundle.receipt.len() > 1024 * 1024 {
            return Err("receipt exceeds 1MiB".into());
        }
        static SCHEMA: std::sync::OnceLock<Result<guard_contract::SchemaValidator, String>> =
            std::sync::OnceLock::new();
        let schema = SCHEMA
            .get_or_init(guard_contract::SchemaValidator::protocol)
            .as_ref()
            .map_err(Clone::clone)?;
        if !schema.validate_bytes(&bundle.receipt)?.is_empty() {
            return Err("receipt violates exact protocol Schema".into());
        }
        let decoded = guard_protocol::decode(&bundle.receipt)?;
        let ProtocolDocument::ApprovalReceipt(receipt) = decoded.document() else {
            return Err("ApprovalReceipt required".into());
        };
        let material = ApprovalMaterial::parse(&bundle.material)?;
        let anchor = self.anchors.current(&receipt.proof.key_id)?;
        if anchor.key_id != receipt.proof.key_id
            || anchor.issuer != receipt.issuer
            || !anchor.identities.iter().any(|identity| {
                identity.principal == receipt.principal
                    && identity.role == receipt.role
                    && identity
                        .purposes
                        .contains(&crate::ApprovalPermission::from(&material.purpose))
                    && identity.intents.contains(&receipt.intent)
            })
            || !anchor.audiences.contains(&material.audience)
            || !anchor.intents.contains(&receipt.intent)
        {
            return Err(
                "issuer/key/principal/role/audience/intent not independently authorized".into(),
            );
        }
        let key =
            VerifyingKey::from_bytes(&anchor.public_key).map_err(|_| "invalid Ed25519 anchor")?;
        if key.is_weak() {
            return Err("weak Ed25519 anchor refused".into());
        }
        let signature = STANDARD
            .decode(receipt.proof.signature.as_str())
            .map_err(|_| "invalid standard base64 signature")?;
        let signature =
            Signature::from_slice(&signature).map_err(|_| "invalid Ed25519 signature")?;
        key.verify_strict(&wire_signing_bytes(&bundle.receipt)?, &signature)
            .map_err(|_| "approval signature verification failed")?;
        if guard_protocol::digest_json(&bundle.material)? != receipt.proof.provenance_digest
            || receipt.scope_digest != material.scope.digest()?
            || material.schema_version != request.material.schema_version
            || material.audience != request.material.audience
            || material.requested_by != request.material.requested_by
            || material.action != request.material.action
            || material.purpose != request.material.purpose
            || material.scope.digest() != request.material.scope.digest()
            || receipt.receipt_id != request.reference
            || receipt.contract_digest != request.contract_digest
            || receipt.intent != request.intent
            || receipt.subject_binding.subject_digest != request.subject_digest
            || receipt.principal == material.requested_by
        {
            return Err(
                "approval exact request/scope/provenance/independent actor mismatch".into(),
            );
        }
        if (receipt.intent == ApprovalIntent::RequirementScope && request.subject_digest.is_some())
            || (receipt.intent != ApprovalIntent::RequirementScope
                && request.subject_digest.is_none())
        {
            return Err("approval kind requires distinct scope/candidate binding".into());
        }
        let now = self.clock.now()?;
        if now.compare_instant(&receipt.issued_at) == Ordering::Less
            || now.compare_instant(&receipt.expires_at) != Ordering::Less
        {
            return Err("approval not current".into());
        }
        let revocation = self.revocations.current(
            &receipt.issuer,
            &receipt.proof.key_id,
            &receipt.receipt_id,
            &receipt.revocation_ref,
        )?;
        if revocation.issuer != receipt.issuer
            || revocation.key_id != receipt.proof.key_id
            || revocation.receipt_id != receipt.receipt_id
            || revocation.revocation_ref != receipt.revocation_ref
            || revocation.authority != anchor.revocation_authority
            || now.compare_instant(&revocation.valid_from) == Ordering::Less
            || now.compare_instant(&revocation.expires_at) != Ordering::Less
        {
            return Err("revocation authority or freshness not proven".into());
        }
        let namespace = format!(
            "{}\0{}\0{}",
            revocation.authority.as_str(),
            receipt.issuer.as_str(),
            receipt.proof.key_id.as_str()
        );
        let record_key = format!(
            "{}\0{}\0{}",
            namespace,
            receipt.receipt_id.as_str(),
            receipt.revocation_ref.as_str()
        );
        // 同receipt不能通过切换名单引用或source身份复活；新批准必须使用新receiptId，换key不能复活旧ID。
        let sticky_key = format!(
            "{}\0{}",
            receipt.issuer.as_str(),
            receipt.receipt_id.as_str()
        );
        let receipt_digest = guard_protocol::digest_json(&bundle.receipt)?;
        let record_digest = guard_protocol::digest_json(
            &serde_json::to_vec(&revocation).map_err(|e| e.to_string())?,
        )?;
        let mut state = self
            .freshness
            .lock()
            .map_err(|_| "authority freshness lock poisoned")?;
        // 慢撤销源不能延长批准；在串行高水位边界重新读取可信时钟。
        let final_now = self.clock.now()?;
        if final_now.compare_instant(&now) == Ordering::Less
            || final_now.compare_instant(&receipt.issued_at) == Ordering::Less
            || final_now.compare_instant(&receipt.expires_at) != Ordering::Less
            || final_now.compare_instant(&revocation.valid_from) == Ordering::Less
            || final_now.compare_instant(&revocation.expires_at) != Ordering::Less
        {
            return Err("approval/revocation expired or clock changed during verification".into());
        }
        let now = final_now;
        if state
            .now
            .as_ref()
            .is_some_and(|previous| now.compare_instant(previous) == Ordering::Less)
        {
            return Err("authority clock moved backwards".into());
        }
        for (entries, key, digest) in [
            (&state.snapshots, &namespace, &revocation.state_digest),
            (&state.records, &record_key, &record_digest),
        ] {
            if entries.get(key).is_some_and(|(revision, previous)| {
                revocation.revision < *revision
                    || (revocation.revision == *revision && digest != previous)
            }) {
                return Err("revocation rollback or same revision equivocation".into());
            }
        }
        // 即使当前已撤销也记住权威版本，不能恢复旧未撤销快照来复活资格。
        state.now = Some(now);
        state
            .snapshots
            .insert(namespace, (revocation.revision, revocation.state_digest));
        state
            .records
            .insert(record_key, (revocation.revision, record_digest));
        if revocation.revoked {
            state.revoked.insert(sticky_key.clone());
        }
        if state.revoked.contains(&sticky_key) {
            return Err("approval/key revoked; observed receipt entry cannot revive".into());
        }
        // 已验签且精确绑定的原始wire才登记；同逻辑ID不能换主体/密钥/期限后重用。
        // 先保留撤销高水位，再隔离冲突；恢复旧字节也不能抹去已观察的权威矛盾。
        if state
            .receipts
            .get(&sticky_key)
            .is_some_and(|previous| previous != &receipt_digest)
        {
            state.receipt_conflicts.insert(sticky_key.clone());
        }
        if state.receipt_conflicts.contains(&sticky_key) {
            return Err(
                "authenticated receipt content changed under the same issuer/receiptId".into(),
            );
        }
        state.receipts.insert(sticky_key, receipt_digest);
        Ok(VerifiedApproval {
            receipt: receipt.clone(),
            material,
        })
    }
}
