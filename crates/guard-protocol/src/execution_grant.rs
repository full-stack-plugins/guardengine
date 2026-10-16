use crate::{
    Action, ActionResource, AuthorityProof, Digest, Extensions, GitOid, Identifier, SchemaVersion,
    UtcTimestamp,
};
use serde::{Deserialize, Serialize};
/// 授予精确动作的短期单次执行声明，不包含通用命令。 无 Java 对应类型。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExecutionGrant {
    /// 冻结协议字段 `schemaVersion`；语义见本 crate 协议表。
    pub schema_version: SchemaVersion,
    /// 冻结协议字段 `grantId`；语义见本 crate 协议表。
    pub grant_id: Identifier,
    /// 冻结协议字段 `operationId`；语义见本 crate 协议表。
    pub operation_id: Identifier,
    /// 冻结协议字段 `decisionDigest`；语义见本 crate 协议表。
    pub decision_digest: Digest,
    /// 冻结协议字段 `principal`；语义见本 crate 协议表。
    pub principal: Identifier,
    /// 冻结协议字段 `audience`；语义见本 crate 协议表。
    pub audience: Identifier,
    /// 冻结协议字段 `action`；语义见本 crate 协议表。
    pub action: Action,
    /// 冻结协议字段 `resource`；语义见本 crate 协议表。
    pub resource: ActionResource,
    /// 冻结协议字段 `subjectDigest`；语义见本 crate 协议表。
    pub subject_digest: Digest,
    /// 冻结协议字段 `expectedTargetOid`；语义见本 crate 协议表。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_target_oid: Option<GitOid>,
    /// 冻结协议字段 `authorizationRef`；语义见本 crate 协议表。
    pub authorization_ref: Identifier,
    /// 冻结协议字段 `issuedAt`；语义见本 crate 协议表。
    pub issued_at: UtcTimestamp,
    /// 冻结协议字段 `expiresAt`；语义见本 crate 协议表。
    pub expires_at: UtcTimestamp,
    /// 冻结协议字段 `nonce`；语义见本 crate 协议表。
    pub nonce: Identifier,
    /// 冻结协议字段 `issuerProof`；语义见本 crate 协议表。
    pub issuer_proof: AuthorityProof,
    /// 冻结协议字段 `extensions`；语义见本 crate 协议表。
    pub extensions: Extensions,
}
