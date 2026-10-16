use crate::{MigrationAttestation, RegisteredPlan};
use guard_protocol::Digest;
/// 显式批准迁移结果，不原地修改旧快照或删除旧证据。无 Java 对应类型。
#[derive(Clone, Debug)]
pub struct MigrationOutcome {
    pub(crate) next: RegisteredPlan,
    pub(crate) previous_digest: Digest,
    pub(crate) approval: MigrationAttestation,
    pub(crate) revalidate: bool,
}
impl MigrationOutcome {
    /// 返回新绑定的只读准备结果。
    pub fn plan(&self) -> &RegisteredPlan {
        &self.next
    }
    /// 返回旧冻结绑定身份以供持久化审计关联。
    pub fn previous_digest(&self) -> &Digest {
        &self.previous_digest
    }
    /// 返回已核验的迁移引用；引用本身不构成执行授权。
    pub fn approval(&self) -> &MigrationAttestation {
        &self.approval
    }
    /// 旧候选证据与新计划不等价时必须重新验证，不得重贴标签。
    pub fn previous_evidence_requires_revalidation(&self) -> bool {
        self.revalidate
    }
}
