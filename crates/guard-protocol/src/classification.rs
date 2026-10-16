use serde::{Deserialize, Serialize};
/// 原始证据与公开摘要的数据分类。 无 Java 对应类型。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Classification {
    /// Private 对应固定协议枚举值。
    Private,
    /// Summary 对应固定协议枚举值。
    Summary,
    /// Public 对应固定协议枚举值。
    Public,
}
