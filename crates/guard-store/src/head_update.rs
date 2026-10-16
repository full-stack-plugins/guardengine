use crate::HeadIdentity;
/// CAS 的机械结果；Conflict 不修改活跃头，不能解释为领域授权。无 Java 对应类型。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HeadUpdate {
    /// 本事务唯一完成的头更新。
    Applied(HeadIdentity),
    /// 完全相同请求重试或对当前同一操作的幂等保持。
    Unchanged(HeadIdentity),
    /// expected 与当前完整令牌不符，返回当前身份供显式重新判断。
    Conflict(Option<HeadIdentity>),
}
