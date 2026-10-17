use guard_protocol::Identifier;
use serde::{Deserialize, Serialize};
/// 只允许收缩当前操作的闭合取消消息，不提供任何执行授权。无Java对应类型。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CancelRequest {
    pub schema_version: String,
    pub repository_id: Identifier,
    pub task_id: Identifier,
    pub operation_id: Identifier,
}
