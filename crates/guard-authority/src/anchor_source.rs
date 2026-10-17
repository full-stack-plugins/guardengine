use crate::AuthorityAnchor;
use guard_protocol::Identifier;
/// 仅允许受保护宿主注入的当前信任锚端口；不得从receipt内的key自选信任。
pub trait AnchorSource: Send + Sync {
    /// 按精确keyId读取当前anchor；缺失、撤销或不可用返回错误。
    fn current(&self, key_id: &Identifier) -> Result<AuthorityAnchor, String>;
}
