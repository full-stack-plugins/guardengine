use crate::{GitSubjectSnapshot, ReleaseArtifactSnapshot};
use serde::{Deserialize, Serialize};
/// Git 候选与发布制品的闭合分型；既有四种 Git wire 保持不变。无 Java 对应类型。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SubjectSnapshot {
    /// 仅表示 worktree/index/commit/merge_candidate。
    Git(Box<GitSubjectSnapshot>),
    /// 独立发布制品身份，不能解读为 Git 提交或工作区。
    ReleaseArtifact(Box<ReleaseArtifactSnapshot>),
}
impl SubjectSnapshot {
    /// 显式读取 Git 分型；发布制品返回 None，消费方必须处理不支持的候选。
    pub fn as_git(&self) -> Option<&GitSubjectSnapshot> {
        match self {
            Self::Git(value) => Some(value),
            Self::ReleaseArtifact(_) => None,
        }
    }
    /// 显式读取发布制品分型；Git 候选返回 None。
    pub fn as_release_artifact(&self) -> Option<&ReleaseArtifactSnapshot> {
        match self {
            Self::Git(_) => None,
            Self::ReleaseArtifact(value) => Some(value),
        }
    }
}
impl From<GitSubjectSnapshot> for SubjectSnapshot {
    fn from(value: GitSubjectSnapshot) -> Self {
        Self::Git(Box::new(value))
    }
}
impl From<ReleaseArtifactSnapshot> for SubjectSnapshot {
    fn from(value: ReleaseArtifactSnapshot) -> Self {
        Self::ReleaseArtifact(Box::new(value))
    }
}
