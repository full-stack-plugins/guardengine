use guard_protocol::{Digest, DomainOwner, Identifier, ProducerIdentity, SubjectKind};
use guard_runtime::SourceInvocation;
/// 独立受保护注册表的精确来源配置；不能从候选JSON反序列化。无Java对应类型。
#[derive(Clone, Debug)]
pub struct ProducerRegistration {
    pub issuer: Identifier,
    pub key_id: Identifier,
    pub public_key: [u8; 32],
    pub revocation_authority: Identifier,
    pub revocation_ref: Identifier,
    pub audience: Identifier,
    pub repository_id: Identifier,
    pub task_id: Identifier,
    pub contract_digest: Digest,
    pub policy_digest: Digest,
    pub scope_digest: Digest,
    pub producer: ProducerIdentity,
    pub domain: DomainOwner,
    pub verifier_pin_digest: Digest,
    pub profile: String,
    pub approved_invocation: SourceInvocation,
    pub allowed_subject_kinds: Vec<SubjectKind>,
    pub max_validity_ms: u64,
}
impl ProducerRegistration {
    pub(crate) fn check(
        &self,
        p: &guard_runtime::DispatchProvenance,
        audience: &Identifier,
    ) -> Result<(), String> {
        if &self.audience != audience
            || self.repository_id != p.scope.repository_id
            || self.task_id != p.scope.task_id
            || self.contract_digest != p.identity.contract_digest
            || self.policy_digest != p.identity.policy_digest
            || self.scope_digest != p.scope.approved_scope_digest
            || self.producer != p.producer
            || self.producer.producer_id != p.verifier_id
            || self.domain != p.domain
            || self.verifier_pin_digest != p.verifier_pin_digest
            || self.profile != p.profile
            || self.approved_invocation != p.approved_invocation
            || !self.allowed_subject_kinds.contains(&p.subject_kind)
            || self.max_validity_ms == 0
            || self.max_validity_ms > 86400000
        {
            return Err("producer is not independently registered for this exact execution scope/profile/pins/argv/environment".into());
        }
        self.approved_invocation
            .verify_effective(&p.effective_invocation, &p.profile)
    }
}
