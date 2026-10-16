use crate::{ExecutionStatus, Identifier, RunSequence, UtcTimestamp};
use serde::{Deserialize, Serialize};
/// 验证器运行事实；序列和谱系须由消费控制端核实。 无 Java 对应类型。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Execution {
    /// 冻结协议字段 `status`；语义见本 crate 协议表。
    pub status: ExecutionStatus,
    /// 冻结协议字段 `exitCode`；语义见本 crate 协议表。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    /// 冻结协议字段 `startedAt`；语义见本 crate 协议表。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<UtcTimestamp>,
    /// 冻结协议字段 `finishedAt`；语义见本 crate 协议表。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<UtcTimestamp>,
    /// 冻结协议字段 `sequence`；语义见本 crate 协议表。
    pub sequence: RunSequence,
    /// 冻结协议字段 `lineageId`；语义见本 crate 协议表。
    pub lineage_id: Identifier,
}
