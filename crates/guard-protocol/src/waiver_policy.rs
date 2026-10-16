use crate::Identifier;
use serde::{Deserialize, Serialize};
/// 例外能力声明，不构成实际批准。 无 Java 对应类型。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WaiverPolicy {
    /// 冻结协议字段 `allowed`；语义见本 crate 协议表。
    pub allowed: bool,
    /// 冻结协议字段 `requiredRole`；语义见本 crate 协议表。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required_role: Option<Identifier>,
    /// 冻结协议字段 `maxAgeSeconds`；语义见本 crate 协议表。
    pub max_age_seconds: u64,
}
