use guard_protocol::{GitOid, Identifier};
use serde::{Deserialize, Serialize};
/// 必须显式选择候选种类；Git OID 不接受自由 ref 表达式。无 Java 对应类型。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Selection {
    /// 捕获当前 Git 可见工作树文件及明确基线。
    Worktree { base: GitOid },
    /// 捕获当前 stage 0 暂存区及明确基线。
    Index { base: GitOid },
    /// 捕获明确提交及基线，与当前暂存内容无关。
    Commit { commit: GitOid, base: GitOid },
    /// 真实合并明确 head 与目标，目标必须仍等于 expected_target。
    MergeCandidate {
        head: GitOid,
        target_ref: Identifier,
        expected_target: GitOid,
    },
}
