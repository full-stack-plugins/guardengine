use guard_protocol::{Action, Digest, Identifier};
use serde::{Deserialize, Serialize};
/// 完整计划身份声明；本地构造不能赋予信任，Bound入口从真实计划产生。无Java对应类型。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PolicyBinding {
    pub repository_id: Identifier,
    pub task_id: Identifier,
    pub subject_digest: Digest,
    pub contract_digest: Digest,
    pub policy_digest: Digest,
    pub plan_digest: Digest,
    pub action: Action,
}
