use crate::ApprovalMaterial;
use guard_protocol::{ApprovalIntent, Digest, Identifier};
/// 调用端请求的精确绑定；请求自身不是权威。无Java对应类型。
#[derive(Clone, Debug)]
pub struct VerificationRequest {
    pub reference: Identifier,
    pub material: ApprovalMaterial,
    pub contract_digest: Digest,
    pub intent: ApprovalIntent,
    pub subject_digest: Option<Digest>,
}
