use guard_protocol::{AuthorityProof, Identifier, UtcTimestamp};
use guard_runtime::DispatchProvenance;
use serde::{Deserialize, Serialize};
/// 闭合来源签名原件，不含规则结论；解析不赋予任何资格。无Java对应类型。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProducerAttestation {
    pub schema_version: String,
    pub attestation_id: Identifier,
    pub issuer: Identifier,
    pub audience: Identifier,
    pub issued_at: UtcTimestamp,
    pub expires_at: UtcTimestamp,
    pub revocation_ref: Identifier,
    pub provenance: DispatchProvenance,
    pub proof: AuthorityProof,
}
