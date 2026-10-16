use serde::{Deserialize, Serialize};
/// 规则严重性，不代表门禁决定。 无 Java 对应类型。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Severity {
    /// Critical 对应固定协议枚举值。
    Critical,
    /// High 对应固定协议枚举值。
    High,
    /// Medium 对应固定协议枚举值。
    Medium,
    /// Low 对应固定协议枚举值。
    Low,
    /// Info 对应固定协议枚举值。
    Info,
}
