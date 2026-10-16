use crate::{Action, GitPath, Identifier};
use serde::{Deserialize, Serialize};
/// 批准任务的仓库、路径和动作范围。 无 Java 对应类型。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TaskScope {
    /// 冻结协议字段 `repositoryIds`；语义见本 crate 协议表。
    pub repository_ids: Vec<Identifier>,
    /// 冻结协议字段 `paths`；语义见本 crate 协议表。
    pub paths: Vec<GitPath>,
    /// 冻结协议字段 `actions`；语义见本 crate 协议表。
    pub actions: Vec<Action>,
}
