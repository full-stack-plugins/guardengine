use guard_protocol::{Digest, Identifier};
use serde::{Deserialize, Serialize};
/// 稳定任务/义务范围；批准范围摘要不等同候选绑定摘要。无Java对应类型。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RunScope {
    pub repository_id: Identifier,
    pub task_id: Identifier,
    pub obligation_id: Identifier,
    pub approved_scope_digest: Digest,
}
