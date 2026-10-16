use crate::{
    Applicability, DomainOwner, Extensions, Identifier, Mode, SchemaVersion, Severity, WaiverPolicy,
};
use serde::{Deserialize, Serialize};
/// 版本化专业规则定义，内核不充当专业域 owner。 无 Java 对应类型。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuleDefinition {
    /// 冻结协议字段 `schemaVersion`；语义见本 crate 协议表。
    pub schema_version: SchemaVersion,
    /// 冻结协议字段 `ruleId`；语义见本 crate 协议表。
    pub rule_id: Identifier,
    /// 冻结协议字段 `revision`；语义见本 crate 协议表。
    pub revision: Identifier,
    /// 冻结协议字段 `owner`；语义见本 crate 协议表。
    pub owner: DomainOwner,
    /// 冻结协议字段 `mode`；语义见本 crate 协议表。
    pub mode: Mode,
    /// 冻结协议字段 `severity`；语义见本 crate 协议表。
    pub severity: Severity,
    /// 冻结协议字段 `applicability`；语义见本 crate 协议表。
    pub applicability: Applicability,
    /// 冻结协议字段 `parameters`；语义见本 crate 协议表。
    pub parameters: serde_json::Value,
    /// 冻结协议字段 `verifierId`；语义见本 crate 协议表。
    pub verifier_id: Identifier,
    /// 冻结协议字段 `requiredEvidence`；语义见本 crate 协议表。
    pub required_evidence: Vec<Identifier>,
    /// 冻结协议字段 `waiverPolicy`；语义见本 crate 协议表。
    pub waiver_policy: WaiverPolicy,
    /// 冻结协议字段 `extensions`；语义见本 crate 协议表。
    pub extensions: Extensions,
}
