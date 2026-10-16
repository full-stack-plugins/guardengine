use serde::{Deserialize, Serialize};
/// 针对预期精确目标集合的覆盖。 无 Java 对应类型。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Coverage {
    /// Complete 对应固定协议枚举值。
    Complete,
    /// Partial 对应固定协议枚举值。
    Partial,
    /// None 对应固定协议枚举值。
    None,
    /// Unknown 对应固定协议枚举值。
    Unknown,
}
