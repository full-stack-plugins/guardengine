use crate::CompileContext;
use guard_protocol::{Action, Digest, Identifier};
/// 对最终编译身份的既有任务授权核验结果；不等同交付验收或 ExecutionGrant。无 Java 对应类型。
#[derive(Clone, Debug)]
pub struct TaskAuthorization {
    /// 外部核验者身份。
    pub authority_id: Identifier,
    /// 必须属于请求列出的已有授权引用。
    pub authorization_ref: Identifier,
    /// 精确编译结果摘要。
    pub contract_digest: Digest,
    /// 精确仓库及任务归属。
    pub context: CompileContext,
    /// 精确准备动作。
    pub action: Action,
}
