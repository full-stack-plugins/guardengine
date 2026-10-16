use crate::{
    ApprovalIntent, AuthorityProof, Digest, Extensions, Identifier, SchemaVersion, SubjectBinding,
    UtcTimestamp,
};
use serde::{Deserialize, Serialize};
/// 需求范围、候选验收或操作授权的外部签名声明。 无 Java 对应类型。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApprovalReceipt {
    /// 冻结协议字段 `schemaVersion`；语义见本 crate 协议表。
    pub schema_version: SchemaVersion,
    /// 冻结协议字段 `receiptId`；语义见本 crate 协议表。
    pub receipt_id: Identifier,
    /// 冻结协议字段 `issuer`；语义见本 crate 协议表。
    pub issuer: Identifier,
    /// 冻结协议字段 `principal`；语义见本 crate 协议表。
    pub principal: Identifier,
    /// 冻结协议字段 `role`；语义见本 crate 协议表。
    pub role: Identifier,
    /// 冻结协议字段 `intent`；语义见本 crate 协议表。
    pub intent: ApprovalIntent,
    /// 冻结协议字段 `scopeDigest`；语义见本 crate 协议表。
    pub scope_digest: Digest,
    /// 冻结协议字段 `contractDigest`；语义见本 crate 协议表。
    pub contract_digest: Digest,
    /// 冻结协议字段 `subjectBinding`；语义见本 crate 协议表。
    pub subject_binding: SubjectBinding,
    /// 冻结协议字段 `issuedAt`；语义见本 crate 协议表。
    pub issued_at: UtcTimestamp,
    /// 冻结协议字段 `expiresAt`；语义见本 crate 协议表。
    pub expires_at: UtcTimestamp,
    /// 冻结协议字段 `revocationRef`；语义见本 crate 协议表。
    pub revocation_ref: Identifier,
    /// 冻结协议字段 `proof`；语义见本 crate 协议表。
    pub proof: AuthorityProof,
    /// 冻结协议字段 `extensions`；语义见本 crate 协议表。
    pub extensions: Extensions,
}
