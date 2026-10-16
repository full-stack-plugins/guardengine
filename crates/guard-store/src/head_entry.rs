use crate::HeadIdentity;
/// 数据库活跃指针及其唯一前驱，仅用于精确请求的幂等重试。无 Java 对应类型。
pub(crate) struct HeadEntry {
    pub identity: HeadIdentity,
    pub previous: Option<HeadIdentity>,
}
