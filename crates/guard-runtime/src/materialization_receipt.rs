use guard_protocol::Digest;
use serde::Serialize;
/// 已核验私有输入清单的本地关联，不是受信签名或OS隔离证明。无Java对应类型。
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MaterializationReceipt {
    pub subject_digest: Digest,
    pub input_manifest_digest: Digest,
    pub receipt_digest: Digest,
}
