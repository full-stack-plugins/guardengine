use serde::{Deserialize, Serialize};
/// 精确工程动作，不包含通用 shell。 无 Java 对应类型。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Action {
    /// BranchCreate 对应固定协议枚举值。
    BranchCreate,
    /// BranchSwitch 对应固定协议枚举值。
    BranchSwitch,
    /// Commit 对应固定协议枚举值。
    Commit,
    /// Push 对应固定协议枚举值。
    Push,
    /// Merge 对应固定协议枚举值。
    Merge,
    /// Release 对应固定协议枚举值。
    Release,
    /// Deploy 对应固定协议枚举值。
    Deploy,
}
