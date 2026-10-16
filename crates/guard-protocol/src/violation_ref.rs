use crate::{Digest, Identifier};
use serde::{Deserialize, Serialize};
/// 原始违规引用，例外不得抹除原结果。 无 Java 对应类型。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ViolationRef {
    /// 冻结协议字段 `ruleId`；语义见本 crate 协议表。
    pub rule_id: Identifier,
    /// 冻结协议字段 `findingId`；语义见本 crate 协议表。
    pub finding_id: Identifier,
    /// 冻结协议字段 `resultDigest`；语义见本 crate 协议表。
    pub result_digest: Digest,
}
