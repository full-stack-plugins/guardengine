use crate::ProducerSigningRequest;
/// 独立受保护签名器端口；候选runner不持有此端口或私钥。无Java对应类型。
pub trait ProducerSigner: Send + Sync {
    /// 仅接收由opaque完成凭据派生的机械请求，返回签名原件；不得签任意报告assessment。
    fn attest(&self, request: &ProducerSigningRequest) -> Result<Vec<u8>, String>;
}
