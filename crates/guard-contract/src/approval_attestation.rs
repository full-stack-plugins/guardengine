use crate::{CompileContext, LayerLevel};
use guard_protocol::{Action, Digest, Identifier};
/// 由调用方配置的独立权威端口返回的核验结果，不提供 JSON 反序列化。无 Java 对应类型。
#[derive(Clone, Debug)]
pub struct ApprovalAttestation {
    /// 核验者身份，由外部受信配置解释。
    pub authority_id: Identifier,
    /// 已确认的批准引用。
    pub approval_ref: Identifier,
    /// 精确层次内容摘要。
    pub contract_digest: Digest,
    /// 适用上下文，不能扩大仓库或任务范围。
    pub context: CompileContext,
    /// 层次归属。
    pub level: LayerLevel,
    /// 本次适用动作。
    pub action: Action,
}
