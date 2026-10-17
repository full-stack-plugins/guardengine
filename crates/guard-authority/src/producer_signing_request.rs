use crate::ProducerAttestation;
use base64::{Engine, engine::general_purpose::STANDARD};
/// 仅由真实完成凭据与独立注册配置准备的机械签名请求，无公开构造器。无Java对应类型。
/// ```compile_fail
/// let _: guard_authority::ProducerSigningRequest = serde_json::from_str("{}").unwrap();
/// ```
pub struct ProducerSigningRequest {
    pub(crate) document: ProducerAttestation,
}
impl ProducerSigningRequest {
    /// 返回固定独立域签名材料，不能替换成任意GuardResult或质量声明。
    pub fn signing_bytes(&self) -> Result<Vec<u8>, String> {
        crate::producer_signing_bytes(
            &serde_json::to_vec(&self.document).map_err(|e| e.to_string())?,
        )
    }
    /// 将独立签名器产生的64字节签名放入原机械请求，不在本库持有私钥。
    pub fn complete(&self, signature: &[u8]) -> Result<Vec<u8>, String> {
        if signature.len() != 64 {
            return Err("64-byte Ed25519 signature required".into());
        }
        let mut doc = self.document.clone();
        doc.proof.signature = guard_protocol::Identifier::new(STANDARD.encode(signature))?;
        serde_json::to_vec(&doc).map_err(|e| e.to_string())
    }
}
