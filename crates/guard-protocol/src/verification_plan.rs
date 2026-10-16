use crate::{
    Action, Digest, ExactTarget, ExecutionBudget, Extensions, Identifier, Obligation, SchemaVersion,
};
use serde::{Deserialize, Serialize};
/// 运行前冻结的候选、契约、策略、义务、精确目标和预算。 无 Java 对应类型。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VerificationPlan {
    /// 冻结协议字段 `schemaVersion`；语义见本 crate 协议表。
    pub schema_version: SchemaVersion,
    /// 冻结协议字段 `planId`；语义见本 crate 协议表。
    pub plan_id: Identifier,
    /// 冻结协议字段 `subjectDigest`；语义见本 crate 协议表。
    pub subject_digest: Digest,
    /// 冻结协议字段 `contractDigest`；语义见本 crate 协议表。
    pub contract_digest: Digest,
    /// 冻结协议字段 `policyDigest`；语义见本 crate 协议表。
    pub policy_digest: Digest,
    /// 冻结协议字段 `action`；语义见本 crate 协议表。
    pub action: Action,
    /// 冻结协议字段 `obligations`；语义见本 crate 协议表。
    pub obligations: Vec<Obligation>,
    /// 冻结协议字段 `targets`；语义见本 crate 协议表。
    pub targets: Vec<ExactTarget>,
    /// 冻结协议字段 `budget`；语义见本 crate 协议表。
    pub budget: ExecutionBudget,
    /// 冻结协议字段 `extensions`；语义见本 crate 协议表。
    pub extensions: Extensions,
}
