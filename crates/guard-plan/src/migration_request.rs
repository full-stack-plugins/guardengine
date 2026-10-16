use guard_protocol::{Digest, Identifier, TaskBinding};
/// 冻结迁移的旧/新身份，供独立控制端精确核验，不支持 Deserialize。无 Java 对应类型。
#[derive(Clone, Debug)]
pub struct MigrationRequest {
    pub(crate) digest: Digest,
    pub(crate) previous_digest: Digest,
    pub(crate) next_digest: Digest,
    pub(crate) previous_binding: TaskBinding,
    pub(crate) next_binding: TaskBinding,
    pub(crate) task_id: Identifier,
    pub(crate) previous_head: guard_store::HeadIdentity,
}
impl MigrationRequest {
    /// 返回包括旧/新计划和完整绑定的批准请求摘要。
    pub fn digest(&self) -> &Digest {
        &self.digest
    }
    /// 返回旧冻结快照摘要。
    pub fn previous_digest(&self) -> &Digest {
        &self.previous_digest
    }
    /// 返回新冻结快照摘要。
    pub fn next_digest(&self) -> &Digest {
        &self.next_digest
    }
    /// 返回旧任务的完整范围和既有授权引用。
    pub fn previous_binding(&self) -> &TaskBinding {
        &self.previous_binding
    }
    /// 返回待独立批准的新绑定，包含新的范围/会话/工作树归属；不能由候选自述替代核验。
    pub fn next_binding(&self) -> &TaskBinding {
        &self.next_binding
    }
    /// 返回保持不变的任务身份。
    pub fn task_id(&self) -> &Identifier {
        &self.task_id
    }
    /// 返回含 revision 的精确旧头，防止批准在竞争或 ABA 后继续使用。
    pub fn previous_head(&self) -> &guard_store::HeadIdentity {
        &self.previous_head
    }
}
