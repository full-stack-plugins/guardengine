use crate::{Digest, Extensions, Identifier, SchemaVersion, SourceRef, TaskScope};
use serde::{Deserialize, Serialize};
/// 需求、批准契约、会话与工作树的任务绑定。 无 Java 对应类型。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TaskBinding {
    /// 冻结协议字段 `schemaVersion`；语义见本 crate 协议表。
    pub schema_version: SchemaVersion,
    /// 冻结协议字段 `taskId`；语义见本 crate 协议表。
    pub task_id: Identifier,
    /// 冻结协议字段 `requirementRefs`；语义见本 crate 协议表。
    pub requirement_refs: Vec<SourceRef>,
    /// 冻结协议字段 `contractDigest`；语义见本 crate 协议表。
    pub contract_digest: Digest,
    /// 冻结协议字段 `scope`；语义见本 crate 协议表。
    pub scope: TaskScope,
    /// 冻结协议字段 `sessionId`；语义见本 crate 协议表。
    pub session_id: Identifier,
    /// 冻结协议字段 `worktreeId`；语义见本 crate 协议表。
    pub worktree_id: Identifier,
    /// 冻结协议字段 `authorizationRefs`；语义见本 crate 协议表。
    pub authorization_refs: Vec<Identifier>,
    /// 冻结协议字段 `dependsOn`；语义见本 crate 协议表。
    pub depends_on: Vec<Identifier>,
    /// 冻结协议字段 `extensions`；语义见本 crate 协议表。
    pub extensions: Extensions,
}
