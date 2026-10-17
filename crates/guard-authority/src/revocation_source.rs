use crate::RevocationSnapshot;
use guard_protocol::Identifier;
/// 独立当前撤销源；网络/存储不可用必须返回错误，不回退缓存。
pub trait RevocationSource: Send + Sync {
    /// 同时核验密钥和精确receipt撤销引用，返回当前快照。
    fn current(
        &self,
        issuer: &Identifier,
        key_id: &Identifier,
        receipt_id: &Identifier,
        revocation_ref: &Identifier,
    ) -> Result<RevocationSnapshot, String>;
}
