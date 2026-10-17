use crate::ProducerAttestation;
/// 独立核验的来源对象，不等同质量通过；每次消费须重新核验。无Java对应类型。
/// ```compile_fail
/// let _: guard_authority::VerifiedProducerEvidence = serde_json::from_str("{}").unwrap();
/// ```
pub struct VerifiedProducerEvidence {
    pub(crate) document: ProducerAttestation,
    pub(crate) raw: Vec<u8>,
}
impl VerifiedProducerEvidence {
    /// 只读机械来源；不输出VERIFIED质量标签或Flow数组。
    pub fn attestation(&self) -> &ProducerAttestation {
        &self.document
    }
    /// 返回原始私有签名原件，供独立证据持久化；不是公开日志。
    pub fn raw(&self) -> &[u8] {
        &self.raw
    }
}
