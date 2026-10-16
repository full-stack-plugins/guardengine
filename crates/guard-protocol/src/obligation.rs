use crate::{ExactTarget, Identifier};
use serde::{Deserialize, Serialize};
/// 执行前冻结的验证义务和精确目标。 无 Java 对应类型。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Obligation {
    /// 冻结协议字段 `obligationId`；语义见本 crate 协议表。
    pub obligation_id: Identifier,
    /// 冻结协议字段 `ruleId`；语义见本 crate 协议表。
    pub rule_id: Identifier,
    /// 冻结协议字段 `verifierId`；语义见本 crate 协议表。
    pub verifier_id: Identifier,
    /// 冻结协议字段 `required`；语义见本 crate 协议表。
    pub required: bool,
    /// 冻结协议字段 `targets`；语义见本 crate 协议表。
    pub targets: Vec<ExactTarget>,
    /// 冻结协议字段 `requiredEvidence`；语义见本 crate 协议表。
    pub required_evidence: Vec<Identifier>,
}
