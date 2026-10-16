use serde::{Deserialize, Serialize};
/// 批准的处置方式，与执行成功或置信度分离。 无 Java 对应类型。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Mode {
    /// Enforce 对应固定协议枚举值。
    Enforce,
    /// Review 对应固定协议枚举值。
    Review,
    /// Advise 对应固定协议枚举值。
    Advise,
}
