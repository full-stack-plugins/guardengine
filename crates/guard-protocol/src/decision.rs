use serde::{Deserialize, Serialize};
/// 独立控制端的决策声明；解析本身不授予任何执行权。 无 Java 对应类型。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Decision {
    /// Allow 对应固定协议枚举值。
    Allow,
    /// AllowWithExceptions 对应固定协议枚举值。
    AllowWithExceptions,
    /// Deny 对应固定协议枚举值。
    Deny,
    /// Incomplete 对应固定协议枚举值。
    Incomplete,
    /// ReviewRequired 对应固定协议枚举值。
    ReviewRequired,
    /// NotApplicable 对应固定协议枚举值。
    NotApplicable,
    /// NotEvaluated 对应固定协议枚举值。
    NotEvaluated,
}
