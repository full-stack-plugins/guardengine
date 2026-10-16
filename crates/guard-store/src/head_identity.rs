use guard_protocol::Digest;
use serde::Serialize;
/// 活跃头的完整比较令牌；revision 防止同操作往返造成 ABA。无 Java 对应类型。
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct HeadIdentity {
    /// 头名称的摘要，不公开原始任务或业务名称。
    pub key_digest: Digest,
    /// 此头从1开始单调递增的存储修订号，不是受信运行序号。
    pub revision: u64,
    /// 已完成不可变操作的摘要。
    pub operation_digest: Digest,
    /// 该操作原始私有字节的摘要。
    pub content_digest: Digest,
}
