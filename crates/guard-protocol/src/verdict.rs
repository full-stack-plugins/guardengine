use serde::{Deserialize, Serialize};
/// 针对规则义务的事实结论，与运行状态分离。 无 Java 对应类型。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Verdict {
    /// Satisfied 对应固定协议枚举值。
    Satisfied,
    /// Violation 对应固定协议枚举值。
    Violation,
    /// Unknown 对应固定协议枚举值。
    Unknown,
    /// NotApplicable 对应固定协议枚举值。
    NotApplicable,
}
