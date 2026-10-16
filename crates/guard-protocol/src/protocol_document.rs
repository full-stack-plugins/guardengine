use crate::{
    ApprovalReceipt, ContractSnapshot, ExecutionGrant, FactEnvelope, GateDecision, GuardResult,
    ProducerIdentity, RuleDefinition, SubjectSnapshot, TaskBinding, VerificationPlan,
};
use serde::{Deserialize, Serialize};
/// v1 文档判别联合；未知文档种类拒绝。无 Java 对应类型。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "documentType")]
pub enum ProtocolDocument {
    /// ContractSnapshot 文档。
    ContractSnapshot(ContractSnapshot),
    /// RuleDefinition 文档。
    RuleDefinition(RuleDefinition),
    /// TaskBinding 文档。
    TaskBinding(TaskBinding),
    /// SubjectSnapshot 文档。
    SubjectSnapshot(SubjectSnapshot),
    /// ProducerIdentity 文档。
    ProducerIdentity(ProducerIdentity),
    /// VerificationPlan 文档。
    VerificationPlan(VerificationPlan),
    /// FactEnvelope 文档。
    FactEnvelope(FactEnvelope),
    /// GuardResult 文档。
    GuardResult(GuardResult),
    /// GateDecision 文档。
    GateDecision(GateDecision),
    /// ApprovalReceipt 文档。
    ApprovalReceipt(ApprovalReceipt),
    /// ExecutionGrant 文档。
    ExecutionGrant(ExecutionGrant),
}
