use serde::{Deserialize, Serialize};
/// Git 仓库对象格式。 无 Java 对应类型。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ObjectFormat {
    /// Sha1 对应固定协议枚举值。
    Sha1,
    /// Sha256 对应固定协议枚举值。
    Sha256,
}
