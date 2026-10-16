use crate::RecordState;
use guard_protocol::Digest;
use serde::Serialize;
/// 公开摘要采用固定白名单；不包含原文、调用者描述或操作 ID 明文。无 Java 对应类型。
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PublicSummary {
    /// 操作 ID 的摘要，仅用于去重，不能证明身份。
    pub operation_digest: Digest,
    /// 私有原文的原始字节摘要。
    pub content_digest: Digest,
    /// 私有原文长度。
    pub size: u64,
    /// 本地完整性状态。
    pub state: RecordState,
}
