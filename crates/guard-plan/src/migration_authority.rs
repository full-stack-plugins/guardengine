use crate::{MigrationAttestation, MigrationRequest};
/// 显式注入的独立批准核验端口；不得使用候选自报布尔值或默认批准实现。
pub trait MigrationAuthority {
    /// 核验迁移来源、范围、期限与撤销并绑定完整请求；本库不签发批准。
    fn verify(&self, request: &MigrationRequest) -> Result<MigrationAttestation, String>;
}
