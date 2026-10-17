use crate::ApprovalPermission;
use guard_protocol::{ApprovalIntent, Identifier};
/// 受保护anchor明确授权的批准主体/角色对；不支持候选反序列化。无Java对应类型。
#[derive(Clone, Debug)]
pub struct PrincipalRole {
    pub principal: Identifier,
    pub role: Identifier,
    pub purposes: Vec<ApprovalPermission>,
    pub intents: Vec<ApprovalIntent>,
}
