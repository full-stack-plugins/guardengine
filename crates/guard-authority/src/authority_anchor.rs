use crate::PrincipalRole;
use guard_protocol::{ApprovalIntent, Identifier};
/// 独立宿主提供的受信签发密钥及权限；声明不证明OS隔离。无Java对应类型。
#[derive(Clone, Debug)]
pub struct AuthorityAnchor {
    pub key_id: Identifier,
    pub revocation_authority: Identifier,
    pub issuer: Identifier,
    pub public_key: [u8; 32],
    pub identities: Vec<PrincipalRole>,
    pub audiences: Vec<Identifier>,
    pub intents: Vec<ApprovalIntent>,
}
