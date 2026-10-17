use crate::{
    AuthorityClock, ProducerRegistration, ProducerRegistry, ProducerSigningRequest,
    RevocationSource, VerifiedProducerEvidence,
};
use base64::{Engine, engine::general_purpose::STANDARD};
use ed25519_dalek::{Signature, VerifyingKey};
use guard_protocol::{AuthorityAlgorithm, AuthorityProof, Identifier, UtcTimestamp};
use guard_runtime::DispatchCompletion;
use std::sync::{Arc, Mutex};
/// 独立核验机械来源，来源真实性和领域质量结论严格分离。无Java对应类型。
pub struct ProducerVerifier {
    registry: Arc<dyn ProducerRegistry>,
    revocations: Arc<dyn RevocationSource>,
    clock: Arc<dyn AuthorityClock>,
    freshness: Mutex<crate::freshness_state::FreshnessState>,
}
impl ProducerVerifier {
    /// 注入受保护注册/撤销/可信时钟；库API不证明部署隔离。
    pub fn new(
        registry: Arc<dyn ProducerRegistry>,
        revocations: Arc<dyn RevocationSource>,
        clock: Arc<dyn AuthorityClock>,
    ) -> Self {
        Self {
            registry,
            revocations,
            clock,
            freshness: Mutex::default(),
        }
    }
    /// 仅从真实opaque完成凭据准备签名，不能传任意GuardResult。
    pub fn prepare(
        &self,
        completion: &DispatchCompletion,
        id: Identifier,
        audience: Identifier,
        expires_at: UtcTimestamp,
    ) -> Result<ProducerSigningRequest, String> {
        let now = self.clock.now()?;
        completion.verify_current(epoch(&now)?)?;
        let p = completion.provenance();
        let registration = self.registry.current(
            &p.producer.producer_id,
            &audience,
            &p.scope.repository_id,
            &p.scope.task_id,
        )?;
        registration.check(p, &audience)?;
        lifetime(&registration, &now, &expires_at, &now)?;
        let provenance_digest =
            guard_protocol::digest_json(&serde_json::to_vec(p).map_err(|e| e.to_string())?)?;
        Ok(ProducerSigningRequest {
            document: crate::ProducerAttestation {
                schema_version: "guard.producer-attestation/v1".into(),
                attestation_id: id,
                issuer: registration.issuer,
                audience,
                issued_at: now,
                expires_at,
                revocation_ref: registration.revocation_ref,
                provenance: p.clone(),
                proof: AuthorityProof {
                    algorithm: AuthorityAlgorithm::Ed25519,
                    key_id: registration.key_id,
                    signature: Identifier::new("pending")?,
                    provenance_digest,
                },
            },
        })
    }
    /// 真实签名核验与当前opaque运行复查；任何来源缺口拒绝。
    pub fn verify_live(
        &self,
        raw: &[u8],
        completion: &DispatchCompletion,
        audience: &Identifier,
    ) -> Result<VerifiedProducerEvidence, String> {
        let document = self.verify_signed(raw, audience)?;
        if &document.provenance != completion.provenance() {
            return Err("signed source differs from actual opaque dispatch".into());
        }
        completion.verify_current(epoch(&self.clock.now()?)?)?;
        // Git/存储复核可能耗时；末次再查注册、撤销和可信时钟。
        self.verify_signed(raw, audience)?;
        Ok(VerifiedProducerEvidence {
            document,
            raw: raw.to_vec(),
        })
    }
    /// 跨重启只消费独立签名原件并重查当前计划/头/双流，不恢复Runtime凭据。
    pub fn verify_restored(
        &self,
        raw: &[u8],
        context: &crate::ProducerRestoreContext<'_>,
        audience: &Identifier,
    ) -> Result<VerifiedProducerEvidence, String> {
        let document = self.verify_signed(raw, audience)?;
        document.provenance.verify_recorded_current(
            context.plan,
            context.contract,
            context.store,
            context.project,
            epoch(&self.clock.now()?)?,
        )?;
        self.verify_signed(raw, audience)?;
        Ok(VerifiedProducerEvidence {
            document,
            raw: raw.to_vec(),
        })
    }
    /// 旧对象不是永久资格；必须重新核验原始签名和当前运行。
    pub fn revalidate_live(
        &self,
        previous: &VerifiedProducerEvidence,
        completion: &DispatchCompletion,
        audience: &Identifier,
    ) -> Result<VerifiedProducerEvidence, String> {
        let verified = self.verify_live(&previous.raw, completion, audience)?;
        if verified.document != previous.document {
            return Err("producer source identity changed".into());
        }
        Ok(verified)
    }
    fn verify_signed(
        &self,
        raw: &[u8],
        audience: &Identifier,
    ) -> Result<crate::ProducerAttestation, String> {
        let document = crate::producer_signing_profile::parse(raw)?;
        let p = &document.provenance;
        let registration = self.registry.current(
            &p.producer.producer_id,
            audience,
            &p.scope.repository_id,
            &p.scope.task_id,
        )?;
        registration.check(p, audience)?;
        if &document.audience != audience
            || document.issuer != registration.issuer
            || document.proof.key_id != registration.key_id
            || document.revocation_ref != registration.revocation_ref
            || document.proof.provenance_digest
                != guard_protocol::digest_json(&serde_json::to_vec(p).map_err(|e| e.to_string())?)?
        {
            return Err("producer signing authority/audience/provenance mismatch".into());
        }
        let key = VerifyingKey::from_bytes(&registration.public_key)
            .map_err(|_| "invalid producer anchor")?;
        if key.is_weak() {
            return Err("weak producer anchor refused".into());
        }
        let signature = Signature::from_slice(
            &STANDARD
                .decode(document.proof.signature.as_str())
                .map_err(|_| "invalid producer base64 signature")?,
        )
        .map_err(|_| "invalid producer signature length")?;
        key.verify_strict(&crate::producer_signing_bytes(raw)?, &signature)
            .map_err(|_| "producer source signature invalid")?;
        lifetime(
            &registration,
            &document.issued_at,
            &document.expires_at,
            &self.clock.now()?,
        )?;
        let revoked = self.revocations.current(
            &document.issuer,
            &document.proof.key_id,
            &document.attestation_id,
            &document.revocation_ref,
        )?;
        if revoked.authority != registration.revocation_authority
            || revoked.issuer != document.issuer
            || revoked.key_id != document.proof.key_id
            || revoked.receipt_id != document.attestation_id
            || revoked.revocation_ref != document.revocation_ref
        {
            return Err("producer revocation source identity mismatch".into());
        }
        let namespace = format!(
            "{}\0{}\0{}",
            revoked.authority.as_str(),
            document.issuer.as_str(),
            document.proof.key_id.as_str()
        );
        let record_key = format!(
            "{}\0{}\0{}",
            namespace,
            document.attestation_id.as_str(),
            document.revocation_ref.as_str()
        );
        let sticky = format!(
            "{}\0{}",
            document.issuer.as_str(),
            document.attestation_id.as_str()
        );
        let record_digest =
            guard_protocol::digest_json(&serde_json::to_vec(&revoked).map_err(|e| e.to_string())?)?;
        let receipt_digest = guard_protocol::digest_json(raw)?;
        let mut state = self
            .freshness
            .lock()
            .map_err(|_| "producer freshness lock poisoned")?;
        let now = self.clock.now()?;
        lifetime(
            &registration,
            &document.issued_at,
            &document.expires_at,
            &now,
        )?;
        if now.compare_instant(&revoked.valid_from).is_lt()
            || now.compare_instant(&revoked.expires_at).is_ge()
            || state
                .now
                .as_ref()
                .is_some_and(|old| now.compare_instant(old).is_lt())
        {
            return Err("producer revocation expired/future or clock rollback".into());
        }
        for (entries, key, digest) in [
            (&state.snapshots, &namespace, &revoked.state_digest),
            (&state.records, &record_key, &record_digest),
        ] {
            if entries.get(key).is_some_and(|(revision, old)| {
                revoked.revision < *revision || (revoked.revision == *revision && digest != old)
            }) {
                return Err("producer revocation rollback/equivocation".into());
            }
        }
        state.now = Some(now);
        state
            .snapshots
            .insert(namespace, (revoked.revision, revoked.state_digest));
        state
            .records
            .insert(record_key, (revoked.revision, record_digest));
        if revoked.revoked {
            state.revoked.insert(sticky.clone());
        }
        if state.revoked.contains(&sticky) {
            return Err("observed producer attestation revoked; new ID required".into());
        }
        if state
            .receipts
            .get(&sticky)
            .is_some_and(|old| old != &receipt_digest)
        {
            state.receipt_conflicts.insert(sticky.clone());
        }
        if state.receipt_conflicts.contains(&sticky) {
            return Err("producer attestation changed under same issuer/ID".into());
        }
        state.receipts.insert(sticky, receipt_digest);
        Ok(document)
    }
}
pub(crate) fn epoch(time: &UtcTimestamp) -> Result<u64, String> {
    u64::try_from(
        chrono::DateTime::parse_from_rfc3339(time.as_str())
            .map_err(|e| e.to_string())?
            .timestamp_millis(),
    )
    .map_err(|_| "nonnegative source clock required".into())
}
fn lifetime(
    registration: &ProducerRegistration,
    issued: &UtcTimestamp,
    expires: &UtcTimestamp,
    now: &UtcTimestamp,
) -> Result<(), String> {
    let (start, end, current) = (epoch(issued)?, epoch(expires)?, epoch(now)?);
    if issued.compare_instant(now).is_gt()
        || now.compare_instant(expires).is_ge()
        || end <= start
        || end - start > registration.max_validity_ms
        || current < start
    {
        return Err("producer attestation expired/future/invalid lifetime".into());
    }
    Ok(())
}
