use crate::ReceiptBundle;
use guard_protocol::Identifier;
/// 受保护宿主的只读批准原件检索端口，不自动批准或签名。
pub trait ReceiptSource: Send + Sync {
    /// 按已有明确引用读取原件；未批准或不可用必须报错。
    fn get(&self, reference: &Identifier) -> Result<ReceiptBundle, String>;
}
