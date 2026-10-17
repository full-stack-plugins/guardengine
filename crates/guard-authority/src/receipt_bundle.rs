/// 来自显式来源的未核验原件；两个字节串都必须重新验证。无Java对应类型。
#[derive(Clone, Debug)]
pub struct ReceiptBundle {
    pub receipt: Vec<u8>,
    pub material: Vec<u8>,
}
