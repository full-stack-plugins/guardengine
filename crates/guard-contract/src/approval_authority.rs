use crate::{ApprovalAttestation, CompileContext, LayerLevel, TaskAuthorization};
use guard_protocol::{Action, Digest, Identifier};
/// 必须由独立受信控制端注入的只读批准核验端口；不得从候选 JSON 构造默认信任。
/// 实现方负责签发者、真实来源、期限和撤销核验；本 crate 不签发信任或自动批准。
pub trait ApprovalAuthority {
    /// 核验某层批准引用与精确摘要、上下文和动作，失败返回原因。
    fn verify_layer(
        &self,
        context: &CompileContext,
        level: LayerLevel,
        approval_ref: &Identifier,
        digest: &Digest,
        action: Action,
    ) -> Result<ApprovalAttestation, String>;
    /// 核验原范围内的已有授权；结果必须绑定最终编译身份，不产生新的操作权限。
    fn authorize_task(
        &self,
        context: &CompileContext,
        digest: &Digest,
        action: Action,
    ) -> Result<TaskAuthorization, String>;
}
