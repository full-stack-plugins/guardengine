use crate::{ApprovalIntent, Digest};
use serde::{Deserialize, Serialize};
/// 批准种类与候选或需求范围绑定。 无 Java 对应类型。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SubjectBinding {
    /// 冻结协议字段 `kind`；语义见本 crate 协议表。
    pub kind: ApprovalIntent,
    /// 冻结协议字段 `subjectDigest`；语义见本 crate 协议表。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject_digest: Option<Digest>,
    /// 冻结协议字段 `scopeDigest`；语义见本 crate 协议表。
    pub scope_digest: Digest,
}
