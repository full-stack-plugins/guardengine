use serde::{Deserialize, Serialize};
/// 运行状态和输入过期观察；Pass仅为local_unverified声明。无Java对应类型。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunState {
    Running,
    Pass,
    Fail,
    Incomplete,
    Cancelled,
    Timeout,
    Error,
    Stale,
}
impl RunState {
    pub(crate) fn terminal(self) -> bool {
        !matches!(self, Self::Running | Self::Stale)
    }
}
