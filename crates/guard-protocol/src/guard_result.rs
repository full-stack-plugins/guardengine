use crate::{
    Assessment, Digest, DomainOwner, EvidenceRef, Execution, Extensions, Identifier, Mode,
    ProducerIdentity, Remediation, SchemaVersion, Severity,
};
use serde::{Deserialize, Serialize};
/// 分离执行状态、规则事实和范围覆盖的专业结果声明。 无 Java 对应类型。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GuardResult {
    /// 冻结协议字段 `schemaVersion`；语义见本 crate 协议表。
    pub schema_version: SchemaVersion,
    /// 冻结协议字段 `resultId`；语义见本 crate 协议表。
    pub result_id: Identifier,
    /// 冻结协议字段 `runId`；语义见本 crate 协议表。
    pub run_id: Identifier,
    /// 冻结协议字段 `operationId`；语义见本 crate 协议表。
    pub operation_id: Identifier,
    /// 冻结协议字段 `taskId`；语义见本 crate 协议表。
    pub task_id: Identifier,
    /// 冻结协议字段 `subjectDigest`；语义见本 crate 协议表。
    pub subject_digest: Digest,
    /// 冻结协议字段 `contractDigest`；语义见本 crate 协议表。
    pub contract_digest: Digest,
    /// 冻结协议字段 `policyDigest`；语义见本 crate 协议表。
    pub policy_digest: Digest,
    /// 冻结协议字段 `producer`；语义见本 crate 协议表。
    pub producer: ProducerIdentity,
    /// 冻结协议字段 `guard`；语义见本 crate 协议表。
    pub guard: DomainOwner,
    /// 冻结协议字段 `ruleId`；语义见本 crate 协议表。
    pub rule_id: Identifier,
    /// 冻结协议字段 `mode`；语义见本 crate 协议表。
    pub mode: Mode,
    /// 冻结协议字段 `severity`；语义见本 crate 协议表。
    pub severity: Severity,
    /// 冻结协议字段 `execution`；语义见本 crate 协议表。
    pub execution: Execution,
    /// 冻结协议字段 `assessment`；语义见本 crate 协议表。
    pub assessment: Assessment,
    /// 冻结协议字段 `evidenceRefs`；语义见本 crate 协议表。
    pub evidence_refs: Vec<EvidenceRef>,
    /// 冻结协议字段 `remediation`；语义见本 crate 协议表。
    pub remediation: Remediation,
    /// 冻结协议字段 `limitations`；语义见本 crate 协议表。
    pub limitations: Vec<Identifier>,
    /// 冻结协议字段 `extensions`；语义见本 crate 协议表。
    pub extensions: Extensions,
}
