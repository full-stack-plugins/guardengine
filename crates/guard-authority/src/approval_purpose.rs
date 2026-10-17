use crate::ApprovalLayer;
use guard_protocol::Digest;
use serde::{Deserialize, Serialize};
/// 签名材料用途，防止层批准、任务授权或迁移互换；无Java对应类型。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ApprovalPurpose {
    Layer { level: ApprovalLayer },
    Task,
    Migration { request_digest: Digest },
}
