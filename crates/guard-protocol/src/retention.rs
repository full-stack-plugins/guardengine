use crate::{Identifier, UtcTimestamp};
use serde::{Deserialize, Serialize};
/// 证据保留策略，不允许公开摘要替代必需原件。 无 Java 对应类型。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Retention {
    /// 冻结协议字段 `policyId`；语义见本 crate 协议表。
    pub policy_id: Identifier,
    /// 冻结协议字段 `retainUntil`；语义见本 crate 协议表。
    pub retain_until: UtcTimestamp,
}
