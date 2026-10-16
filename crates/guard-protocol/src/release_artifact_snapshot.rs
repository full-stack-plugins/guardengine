use crate::{
    Digest, Extensions, Identifier, ReleaseArtifactIdentity, ReleaseArtifactKind,
    ReleaseBuildIdentity, ReleaseSourceIdentity, SchemaVersion,
};
use serde::{Deserialize, Serialize};
/// 与 Git 候选不混用的发布制品快照；不是实际采集、发布或签名证明。无 Java 对应类型。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReleaseArtifactSnapshot {
    /// 当前固定协议版本。
    pub schema_version: SchemaVersion,
    /// 唯一 release_artifact 标签。
    pub kind: ReleaseArtifactKind,
    /// 外部登记的源码仓库 ID，不推断本地路径或 remote URL。
    pub repository_id: Identifier,
    /// 原始制品内容身份，与 Git 来源分开。
    pub artifact: ReleaseArtifactIdentity,
    /// 不可变 Git 来源声明。
    pub source: ReleaseSourceIdentity,
    /// 精确构建运行和工具声明。
    pub build: ReleaseBuildIdentity,
    /// 本次制品验证输入清单摘要，不能代替来源/构建清单原件。
    pub input_manifest_digest: Digest,
    /// 仅允许命名空间元数据，不能改变制品或来源身份。
    pub extensions: Extensions,
}
