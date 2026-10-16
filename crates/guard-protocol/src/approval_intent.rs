use serde::{Deserialize, Serialize};
/// 需求范围批准、候选验收和操作授权严格区分。 无 Java 对应类型。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ApprovalIntent {
    /// RequirementScope 对应固定协议枚举值。
    RequirementScope,
    /// CandidateAcceptance 对应固定协议枚举值。
    CandidateAcceptance,
    /// OperationAuthorization 对应固定协议枚举值。
    OperationAuthorization,
}
