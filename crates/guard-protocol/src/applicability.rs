use crate::Identifier;
use serde::{Deserialize, Serialize};
/// 显式适用条件及有界参数，由契约编译器解释。 无 Java 对应类型。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Applicability {
    /// 冻结协议字段 `predicate`；语义见本 crate 协议表。
    pub predicate: Identifier,
    /// 冻结协议字段 `parameters`；语义见本 crate 协议表。
    pub parameters: serde_json::Value,
}
