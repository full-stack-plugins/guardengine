use guard_protocol::{Digest, Identifier};
use serde::{Deserialize, Serialize};
/// 调用端显式提供的完整绑定；仓库/任务身份应由外部注册表核实。无 Java 对应类型。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureBinding {
    /// 声明的仓库身份，不根据路径、remote URL 或摘要推导。
    pub repository_id: Identifier,
    /// 声明的任务身份。
    pub task_id: Identifier,
    /// 声明的工作树身份。
    pub worktree_id: Identifier,
    /// 已冻结契约内容身份。
    pub contract_digest: Digest,
    /// 已冻结策略内容身份。
    pub policy_digest: Digest,
    /// 验证器及配置身份；本库不核验声明来源。
    pub verifier_digest: Digest,
    /// 工具链及环境身份；本库不核验声明来源。
    pub environment_digest: Digest,
}
