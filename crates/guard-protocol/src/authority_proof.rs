use crate::{AuthorityAlgorithm, Digest, Identifier};
use serde::{Deserialize, Serialize};
/// 外部签发者的签名与来源声明；协议解析绝不代表验签成功。 无 Java 对应类型。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AuthorityProof {
    /// 冻结协议字段 `algorithm`；语义见本 crate 协议表。
    pub algorithm: AuthorityAlgorithm,
    /// 冻结协议字段 `keyId`；语义见本 crate 协议表。
    pub key_id: Identifier,
    /// 冻结协议字段 `signature`；语义见本 crate 协议表。
    pub signature: Identifier,
    /// 冻结协议字段 `provenanceDigest`；语义见本 crate 协议表。
    pub provenance_digest: Digest,
}
