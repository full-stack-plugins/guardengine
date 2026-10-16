use crate::{Digest, GitPath, Identifier};
use serde::{Deserialize, Serialize};
/// 输入清单中的精确目标，不允许隐式扩大或缩小。 无 Java 对应类型。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExactTarget {
    /// 冻结协议字段 `repositoryId`；语义见本 crate 协议表。
    pub repository_id: Identifier,
    /// 冻结协议字段 `path`；语义见本 crate 协议表。
    pub path: GitPath,
    /// 冻结协议字段 `selector`；语义见本 crate 协议表。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selector: Option<Identifier>,
    /// 冻结协议字段 `inputDigest`；语义见本 crate 协议表。
    pub input_digest: Digest,
}
