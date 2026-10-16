use crate::{Digest, Extensions, GitOid, Identifier, ObjectFormat, SchemaVersion, SubjectKind};
use serde::{Deserialize, Serialize};
/// 四种 Git 候选的闭合 wire，不能接收发布制品字段或冒用制品身份。 无 Java 对应类型。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GitSubjectSnapshot {
    /// 冻结协议字段 `schemaVersion`；语义见本 crate 协议表。
    pub schema_version: SchemaVersion,
    /// 冻结协议字段 `kind`；语义见本 crate 协议表。
    pub kind: SubjectKind,
    /// 冻结协议字段 `repositoryId`；语义见本 crate 协议表。
    pub repository_id: Identifier,
    /// 冻结协议字段 `worktreeId`；语义见本 crate 协议表。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub worktree_id: Option<Identifier>,
    /// 冻结协议字段 `objectFormat`；语义见本 crate 协议表。
    pub object_format: ObjectFormat,
    /// 冻结协议字段 `base`；语义见本 crate 协议表。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base: Option<GitOid>,
    /// 冻结协议字段 `head`；语义见本 crate 协议表。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub head: Option<GitOid>,
    /// 冻结协议字段 `tree`；语义见本 crate 协议表。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tree: Option<GitOid>,
    /// 冻结协议字段 `targetRef`；语义见本 crate 协议表。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_ref: Option<Identifier>,
    /// 冻结协议字段 `expectedTargetOid`；语义见本 crate 协议表。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_target_oid: Option<GitOid>,
    /// 冻结协议字段 `inputManifestDigest`；语义见本 crate 协议表。
    pub input_manifest_digest: Digest,
    /// 冻结协议字段 `diffDigest`；语义见本 crate 协议表。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff_digest: Option<Digest>,
    /// 冻结协议字段 `extensions`；语义见本 crate 协议表。
    pub extensions: Extensions,
}
