use crate::{PlanningOutcome, freeze};
use guard_contract::CompiledContract;
use guard_protocol::{ExecutionBudget, TaskBinding};
use guard_store::EvidenceStore;
use guard_subject::CapturedSubject;
use std::path::Path;
/// 从私有存储复核原始快照，必须提供仍有效的不可变编译产物与真实候选重新构造。
/// 解析持久 JSON 不产生 FrozenPlan；参数变化、坏原件、旧候选或不一致记录一律拒绝。
#[allow(clippy::too_many_arguments)]
pub fn load(
    store: &EvidenceStore,
    operation: &str,
    now_ms: u64,
    project: &Path,
    contract: &CompiledContract,
    binding: &TaskBinding,
    subject: &CapturedSubject,
    budget: ExecutionBudget,
) -> Result<PlanningOutcome, String> {
    let bytes = store.read_private(operation, now_ms)?;
    let reconstructed = freeze(project, contract, binding, subject, budget)?;
    if guard_protocol::canonicalize(&bytes)?
        != guard_protocol::canonicalize(
            &serde_json::to_vec(&reconstructed.snapshot).map_err(|e| e.to_string())?,
        )?
    {
        return Err(
            "persisted approval binding differs from independently reconstructed plan".into(),
        );
    }
    Ok(reconstructed)
}
