use serde::{Deserialize, Serialize};
/// 只读图谱事实种类，不包含门禁权限。 无 Java 对应类型。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FactType {
    /// Symbol 对应固定协议枚举值。
    Symbol,
    /// Edge 对应固定协议枚举值。
    Edge,
    /// SourceSpan 对应固定协议枚举值。
    SourceSpan,
}
