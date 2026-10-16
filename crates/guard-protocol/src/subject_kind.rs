use serde::{Deserialize, Serialize};
/// 候选种类；暂存区、提交和最终合入候选不能互用。 无 Java 对应类型。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubjectKind {
    /// Worktree 对应固定协议枚举值。
    Worktree,
    /// Index 对应固定协议枚举值。
    Index,
    /// Commit 对应固定协议枚举值。
    Commit,
    /// MergeCandidate 对应固定协议枚举值。
    MergeCandidate,
}
