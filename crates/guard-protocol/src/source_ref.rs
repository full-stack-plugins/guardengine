use crate::{Digest, GitPath, Identifier};
use serde::{Deserialize, Serialize};
/// 批准规格或规则的不可变来源。 无 Java 对应类型。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceRef {
    /// 冻结协议字段 `repoId`；语义见本 crate 协议表。
    pub repo_id: Identifier,
    /// 冻结协议字段 `path`；语义见本 crate 协议表。
    pub path: GitPath,
    /// 冻结协议字段 `ref`；语义见本 crate 协议表。
    pub r#ref: Identifier,
    /// 冻结协议字段 `contentDigest`；语义见本 crate 协议表。
    pub content_digest: Digest,
}
