use serde::{Deserialize, Serialize};
/// 声明的签名算法；实际验签必须由独立消费控制端执行。 无 Java 对应类型。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AuthorityAlgorithm {
    /// Ed25519 对应固定协议枚举值。
    Ed25519,
}
