use guard_protocol::Digest;
use serde::Serialize;
/// 私有原件内容引用；原始输入/双流独立保存，不编码成巨大JSON数组。无Java对应类型。
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PolicyArtifactRef {
    pub operation: String,
    pub digest: Digest,
    pub size: u64,
}
