use crate::PolicyArtifactRef;
use guard_protocol::{ExecutionStatus, UtcTimestamp};
use serde::Serialize;
/// K06真实进程阶段观察；未派发保持无时间，不由OPA结果补造。无Java对应类型。
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PolicyProcessObservation {
    pub status: ExecutionStatus,
    pub exit_code: Option<i32>,
    pub started_at: Option<UtcTimestamp>,
    pub finished_at: Option<UtcTimestamp>,
    pub stop_reason: String,
    pub output_truncated: bool,
    pub stdout: PolicyArtifactRef,
    pub stderr: PolicyArtifactRef,
}
