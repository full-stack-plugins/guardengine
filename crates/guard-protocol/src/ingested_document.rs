use crate::{ProtocolDocument, TrustLevel};
/// 解析后的不受信声明，不能直接作为动作授权。无 Java 对应类型。
#[derive(Clone, Debug)]
pub struct IngestedDocument {
    pub(crate) document: ProtocolDocument,
    pub(crate) original: Vec<u8>,
}
impl IngestedDocument {
    /// 只读访问声明内容；返回值仍未证明生产者、批准或对象真实性。
    pub fn document(&self) -> &ProtocolDocument {
        &self.document
    }
    /// 原始输入用于审计和内容校验，不用重序列化替代原件。
    pub fn original_bytes(&self) -> &[u8] {
        &self.original
    }
    /// 协议解析不能自行升级来源信任，固定返回未验证。
    pub fn trust_level(&self) -> TrustLevel {
        TrustLevel::Unverified
    }
}
