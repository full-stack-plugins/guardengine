use guard_protocol::Digest;
use serde::Serialize;
/// 活跃头追加审计；只公开摘要、固定代码和调用者时间。无 Java 对应类型。
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct HeadAuditEvent {
    /// 本地 SQLite 事件序号，不是受信执行顺序。
    pub sequence: u64,
    /// 头名称摘要。
    pub key_digest: Digest,
    /// 本次尝试使用的操作摘要。
    pub operation_digest: Digest,
    /// 固定代码 applied 或 conflict。
    pub code: String,
    /// 调用者提供的时间。
    pub at_ms: u64,
}
