use guard_protocol::Digest;
use serde::Serialize;
/// 私有原始字节原件引用；公开摘要不得直接展示原始输出。无Java对应类型。
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactRef {
    pub operation: String,
    pub digest: Digest,
    pub size: u64,
}
