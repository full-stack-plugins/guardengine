use crate::{Identifier, TargetSelection};
use serde::{Deserialize, Serialize};

/// 批准契约中的完整验证义务及目标选择范围，区别于已绑定候选的计划义务。无 Java 对应类型。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContractObligation {
    /// 稳定义务身份。
    pub obligation_id: Identifier,
    /// 已批准专业规则身份。
    pub rule_id: Identifier,
    /// 已批准验证器身份。
    pub verifier_id: Identifier,
    /// 是否为必需义务，绑定当前候选不能降低此标记。
    pub required: bool,
    /// 已批准目标选择全集；执行计划必须逐项绑定，不能漏目标。
    pub targets: Vec<TargetSelection>,
    /// 已批准证据类型全集。
    pub required_evidence: Vec<Identifier>,
}
