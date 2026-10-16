use crate::{
    Action, Decision, Digest, Extensions, Identifier, SchemaVersion, UtcTimestamp, ViolationRef,
};
use serde::{Deserialize, Serialize};
/// 控制端签发的门禁声明；输入解析不产生授权。 无 Java 对应类型。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GateDecision {
    /// 冻结协议字段 `schemaVersion`；语义见本 crate 协议表。
    pub schema_version: SchemaVersion,
    /// 冻结协议字段 `decisionId`；语义见本 crate 协议表。
    pub decision_id: Identifier,
    /// 冻结协议字段 `action`；语义见本 crate 协议表。
    pub action: Action,
    /// 冻结协议字段 `subjectDigest`；语义见本 crate 协议表。
    pub subject_digest: Digest,
    /// 冻结协议字段 `contractDigest`；语义见本 crate 协议表。
    pub contract_digest: Digest,
    /// 冻结协议字段 `policyDigest`；语义见本 crate 协议表。
    pub policy_digest: Digest,
    /// 冻结协议字段 `planDigest`；语义见本 crate 协议表。
    pub plan_digest: Digest,
    /// 冻结协议字段 `resultDigests`；语义见本 crate 协议表。
    pub result_digests: Vec<Digest>,
    /// 冻结协议字段 `approvalRefs`；语义见本 crate 协议表。
    pub approval_refs: Vec<Identifier>,
    /// 冻结协议字段 `decision`；语义见本 crate 协议表。
    pub decision: Decision,
    /// 冻结协议字段 `reasons`；语义见本 crate 协议表。
    pub reasons: Vec<Identifier>,
    /// 冻结协议字段 `missingObligations`；语义见本 crate 协议表。
    pub missing_obligations: Vec<Identifier>,
    /// 冻结协议字段 `activeViolations`；语义见本 crate 协议表。
    pub active_violations: Vec<ViolationRef>,
    /// 冻结协议字段 `requiredActions`；语义见本 crate 协议表。
    pub required_actions: Vec<Action>,
    /// 冻结协议字段 `evaluatedAt`；语义见本 crate 协议表。
    pub evaluated_at: UtcTimestamp,
    /// 冻结协议字段 `expiresAt`；语义见本 crate 协议表。
    pub expires_at: UtcTimestamp,
    /// 冻结协议字段 `issuer`；语义见本 crate 协议表。
    pub issuer: Identifier,
    /// 冻结协议字段 `extensions`；语义见本 crate 协议表。
    pub extensions: Extensions,
}
