use guard_protocol::Digest;
use serde::Serialize;
/// 只含白名单代码和摘要的追加审计；不记录私有内容。无 Java 对应类型。
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AuditEvent {
    /// SQLite 单调审计序号，只表征本库存储顺序，不能当作受信运行序号。
    pub sequence: u64,
    /// 操作摘要，避免公开原始 ID。
    pub operation_digest: Digest,
    /// 固定代码：intent/completed/incomplete_recovered/artifact_invalid/expired。
    pub code: String,
    /// 调用者提供的时间；存储层不声明它来自可信时钟。
    pub at_ms: u64,
}
