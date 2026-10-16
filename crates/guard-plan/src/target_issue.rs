use guard_protocol::{Identifier, TargetSelection};
use serde::Serialize;
/// 逐义务缺口；无原件时不伪造 ExactTarget 摘要。无 Java 对应类型。
#[derive(Clone, Debug, Serialize)]
pub struct TargetIssue {
    /// 缺口属于哪条冻结义务。
    pub obligation_id: Identifier,
    /// 不丢失批准的原始选择和保护摘要。
    pub selection: TargetSelection,
    /// 固定原因代码：missing_input 或 protected_input_changed。
    pub code: String,
}
