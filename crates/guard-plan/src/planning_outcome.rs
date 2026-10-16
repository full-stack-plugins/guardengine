use crate::{FrozenPlan, PlanStatus, TargetIssue};
use guard_contract::CompiledContract;
use guard_protocol::{ContractObligation, Digest, TaskBinding};
use guard_store::{EvidenceStore, PublicSummary};
use guard_subject::CapturedSubject;
use serde_json::Value;
/// 不可变批准绑定与准备结果；缺口不删义务、不输出局部可执行成功。无 Java 对应类型。
#[derive(Clone, Debug)]
pub struct PlanningOutcome {
    pub(crate) contract: CompiledContract,
    pub(crate) binding: TaskBinding,
    pub(crate) plan: Option<FrozenPlan>,
    pub(crate) issues: Vec<TargetIssue>,
    pub(crate) snapshot: Value,
    pub(crate) digest: Digest,
    pub(crate) subject: CapturedSubject,
    pub(crate) baseline: Value,
    pub(crate) project: std::path::PathBuf,
}
impl PlanningOutcome {
    /// 只有全部预期目标存在且保护输入相符才是 Ready；不是工程通过。
    pub fn status(&self) -> PlanStatus {
        if self.plan.is_some() {
            PlanStatus::Ready
        } else {
            PlanStatus::Incomplete
        }
    }
    /// 返回完整冻结运行计划；不完整准备绝不暴露缩减后的运行计划。
    pub fn plan(&self) -> Option<&FrozenPlan> {
        self.plan.as_ref()
    }
    /// 返回批准契约的完整选择义务，不根据工具结果或模块存在性删减。
    pub fn contract_obligations(&self) -> &[ContractObligation] {
        self.contract.obligations()
    }
    /// 返回原 TaskBinding 的只读引用。
    pub fn binding(&self) -> &TaskBinding {
        &self.binding
    }
    /// 返回精确缺失或保护输入漂移，供修复准备缺口。
    pub fn issues(&self) -> &[TargetIssue] {
        &self.issues
    }
    /// 返回完整批准绑定/真实候选/准备结果的规范内容摘要。
    pub fn digest(&self) -> &Digest {
        &self.digest
    }
    /// 持久化原始冻结绑定及批准引用，使用 GuardStore 私有原件与幂等写入。
    /// 保留期和时间只是调用者配置，不签名、授信或签发操作许可。
    pub fn persist(
        &self,
        store: &EvidenceStore,
        operation: &str,
        retain_until_ms: u64,
        now_ms: u64,
    ) -> Result<PublicSummary, String> {
        let bytes = guard_protocol::canonicalize(
            &serde_json::to_vec(&self.snapshot).map_err(|e| e.to_string())?,
        )?;
        store.put(operation, &bytes, retain_until_ms, now_ms)
    }
    /// 首次以 CAS 注册已批准绑定，或只复用完全相同的 active baseline。
    /// 已有不同契约的同任务不能用此入口重绑，必须显式 migrate。
    pub fn register(
        &self,
        store: &EvidenceStore,
        retain_until_ms: u64,
        now_ms: u64,
    ) -> Result<crate::RegisteredPlan, String> {
        crate::registration::register(self, store, retain_until_ms, now_ms)
    }
}
