use crate::{Digest, Identifier};
use serde::{Deserialize, Serialize};
/// 图谱索引身份及其源候选摘要。 无 Java 对应类型。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IndexIdentity {
    /// 冻结协议字段 `indexId`；语义见本 crate 协议表。
    pub index_id: Identifier,
    /// 冻结协议字段 `digest`；语义见本 crate 协议表。
    pub digest: Digest,
    /// 冻结协议字段 `subjectDigest`；语义见本 crate 协议表。
    pub subject_digest: Digest,
}
