use crate::{RunIdentity, RunScope, RunState};
use guard_protocol::{Digest, Identifier, RunSequence};
use serde::{Deserialize, Serialize};
/// 私有原件中的完整状态；只通过受管CAS成为当前头。无Java对应类型。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RunRecord {
    pub schema_version: String,
    pub scope: RunScope,
    pub identity: RunIdentity,
    pub sequence: RunSequence,
    pub operation_id: Identifier,
    pub state: RunState,
    pub retain_until_ms: u64,
    pub result_operation: Option<String>,
    pub result_digest: Option<Digest>,
}
