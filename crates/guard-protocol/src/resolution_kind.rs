use serde::{Deserialize, Serialize};
/// 符号或边的解析方式，未知不能证明不存在依赖。 无 Java 对应类型。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ResolutionKind {
    /// Static 对应固定协议枚举值。
    Static,
    /// DynamicResolved 对应固定协议枚举值。
    DynamicResolved,
    /// DynamicUnknown 对应固定协议枚举值。
    DynamicUnknown,
    /// Unresolved 对应固定协议枚举值。
    Unresolved,
}
