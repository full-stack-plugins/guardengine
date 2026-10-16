use crate::{Digest, Identifier, TrustLevel};
/// 保留旧协议原件的显式适配观察，绝不投影成工程通过。无 Java 对应类型。
#[derive(Clone, Debug)]
pub struct LegacyObservation {
    pub(crate) original: Vec<u8>,
    pub(crate) adapter: Identifier,
    pub(crate) digest: Digest,
}
impl LegacyObservation {
    /// 返回原始字节，保留空白、字段和旧生产者声明。
    pub fn original_bytes(&self) -> &[u8] {
        &self.original
    }
    /// 返回显式适配器版本，不自动选择 latest。
    pub fn adapter_version(&self) -> &str {
        self.adapter.as_str()
    }
    /// 返回原始报告摘要，不用投影内容重算原件身份。
    pub fn original_digest(&self) -> &Digest {
        &self.digest
    }
    /// 旧输入不自动获得来源信任。
    pub fn trust_level(&self) -> TrustLevel {
        TrustLevel::Unverified
    }
    /// 原件中的 allow/success/exit 0 不转成工程放行。
    pub fn decision(&self) -> &'static str {
        "NOT_EVALUATED"
    }
}
