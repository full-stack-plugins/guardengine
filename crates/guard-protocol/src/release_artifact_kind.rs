use serde::{Deserialize, Serialize};
/// 发布制品的唯一分型标签，不接受四种 Git kind。无 Java 对应类型。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReleaseArtifactKind {
    /// 独立制品候选，不能拿 commitSha 代替原始制品字节摘要。
    ReleaseArtifact,
}
