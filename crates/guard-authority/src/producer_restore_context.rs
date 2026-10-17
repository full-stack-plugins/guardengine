use guard_contract::CompiledContract;
use guard_plan::RegisteredPlan;
use guard_store::EvidenceStore;
use std::path::Path;
/// 跨重启签名消费的当前上下文；它本身不授信，也不恢复DispatchCompletion。无Java对应类型。
pub struct ProducerRestoreContext<'a> {
    pub plan: &'a RegisteredPlan,
    pub contract: &'a CompiledContract,
    pub store: &'a EvidenceStore,
    pub project: &'a Path,
}
