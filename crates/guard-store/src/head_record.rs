use crate::HeadIdentity;
/// 同一次受锁事务内核验的活跃头和私有原件；不赋予任何领域权限。无 Java 对应类型。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HeadRecord {
    /// 防止跨头替换及 ABA 的比较令牌。
    pub identity: HeadIdentity,
    /// 已核验长度、摘要、完整状态与保留期的私有原始字节。
    pub bytes: Vec<u8>,
}
