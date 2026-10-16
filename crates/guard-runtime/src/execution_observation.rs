use crate::{ArtifactRef, InvocationReceipt};
use guard_protocol::{Digest, ExecutionStatus, Identifier, RunSequence, UtcTimestamp};
use guard_run_ledger::{FinalizeOutcome, RunIdentity, RunState};
use serde::Serialize;
/// 本地执行观察；没有qualityPass或可信生产者字段。无Java对应类型。
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionObservation {
    pub schema_version: &'static str,
    pub obligation_id: Identifier,
    pub verifier_id: Identifier,
    pub identity: RunIdentity,
    pub plan_digest: Digest,
    pub invocation: InvocationReceipt,
    pub sequence: RunSequence,
    pub execution_status: ExecutionStatus,
    pub exit_code: Option<i32>,
    pub started_at: Option<UtcTimestamp>,
    pub finished_at: Option<UtcTimestamp>,
    pub stop_reason: String,
    pub output_truncated: bool,
    pub input_drifted: bool,
    pub recheck_error: Option<String>,
    pub state: RunState,
    pub stdout: ArtifactRef,
    pub stderr: ArtifactRef,
    pub trust: &'static str,
    /// 小metadata原件的终结结果；不进入被保存的metadata，防止自引用。
    #[serde(skip)]
    pub finalization: Option<FinalizeOutcome>,
}
