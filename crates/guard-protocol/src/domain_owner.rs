use serde::{Deserialize, Serialize};
/// 专业域唯一归属；通用内核不能自称专业规则 owner。 无 Java 对应类型。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DomainOwner {
    /// Specguard 对应固定协议枚举值。
    Specguard,
    /// Archguard 对应固定协议枚举值。
    Archguard,
    /// Codeguard 对应固定协议枚举值。
    Codeguard,
    /// Testguard 对应固定协议枚举值。
    Testguard,
    /// Gitguard 对应固定协议枚举值。
    Gitguard,
    /// Flowguard 对应固定协议枚举值。
    Flowguard,
}
