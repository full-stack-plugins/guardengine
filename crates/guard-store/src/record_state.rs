use serde::Serialize;
/// 本地完整性状态；Complete 绝不代表可信生产者或工程通过。无 Java 对应类型。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RecordState {
    /// 已持久化意图，原件尚未完成。
    Pending,
    /// 事务与原件均完成；读取仍须重新校验。
    Complete,
    /// 中断、缺件或损坏，保留原始恢复线索。
    Quarantined,
    /// 保留期已结束，元数据保留且不能再次使用相同操作 ID。
    Expired,
}
impl RecordState {
    pub(crate) fn parse(value: &str) -> Result<Self, String> {
        match value {
            "pending" => Ok(Self::Pending),
            "complete" => Ok(Self::Complete),
            "quarantined" => Ok(Self::Quarantined),
            "expired" => Ok(Self::Expired),
            _ => Err("unrecognized stored state".into()),
        }
    }
}
