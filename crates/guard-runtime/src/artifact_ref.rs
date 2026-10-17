use guard_protocol::Digest;
use serde::{Deserialize, Serialize};
/// 私有原始字节原件引用；公开摘要不得直接展示原始输出。无Java对应类型。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ArtifactRef {
    pub operation: String,
    pub digest: Digest,
    pub size: u64,
}
