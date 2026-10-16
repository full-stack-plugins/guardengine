use serde::{Deserialize, Serialize};
/// 运行状态，超时和取消不抹除已发现违规。 无 Java 对应类型。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ExecutionStatus {
    /// Completed 对应固定协议枚举值。
    Completed,
    /// Failed 对应固定协议枚举值。
    Failed,
    /// TimedOut 对应固定协议枚举值。
    TimedOut,
    /// Cancelled 对应固定协议枚举值。
    Cancelled,
    /// NotRun 对应固定协议枚举值。
    NotRun,
}
