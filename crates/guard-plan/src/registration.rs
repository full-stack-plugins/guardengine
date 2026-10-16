use crate::{PlanningOutcome, RegisteredPlan, load};
use guard_contract::CompiledContract;
use guard_protocol::{ExecutionBudget, TaskBinding};
use guard_store::{EvidenceStore, HeadUpdate};
use guard_subject::CapturedSubject;
use serde_json::json;
use std::path::Path;
pub(crate) fn baseline_bytes(outcome: &PlanningOutcome) -> Result<Vec<u8>, String> {
    guard_protocol::canonicalize(&serde_json::to_vec(&outcome.baseline).map_err(|e| e.to_string())?)
}
pub(crate) fn head_key(outcome: &PlanningOutcome) -> Result<String, String> {
    // 不把可自报的 session/worktree 放入 key，避免同一任务换标识绕过已有批准绑定。
    let digest = guard_protocol::digest_json(
        &serde_json::to_vec(&json!({"schemaVersion":"guard.task-head/v1",
        "repository":outcome.contract.context().repository_id,"task":outcome.binding.task_id}))
        .map_err(|e| e.to_string())?,
    )?;
    Ok(format!("guard-plan:task:{}", digest.as_str()))
}
pub(crate) fn put_baseline(
    outcome: &PlanningOutcome,
    store: &EvidenceStore,
    until: u64,
    now: u64,
) -> Result<String, String> {
    let bytes = baseline_bytes(outcome)?;
    let digest = guard_protocol::digest_json(&bytes)?;
    let operation = format!("guard-plan:baseline:{}", digest.as_str());
    store.put(&operation, &bytes, until, now)?;
    Ok(operation)
}
pub(crate) fn register(
    outcome: &PlanningOutcome,
    store: &EvidenceStore,
    until: u64,
    now: u64,
) -> Result<RegisteredPlan, String> {
    if outcome.plan.is_none() {
        return Err("incomplete draft cannot register an execution plan".into());
    }
    guard_subject::verify_current(
        &outcome.project,
        &outcome.subject.binding,
        &outcome.subject.selection,
        &outcome.subject,
    )?;
    let key = head_key(outcome)?;
    let bytes = baseline_bytes(outcome)?;
    let head = if let Some(existing) = store.read_head(&key, now)? {
        if existing.bytes != bytes {
            return Err("task already bound to another baseline; explicit independently approved migration required".into());
        }
        existing.identity
    } else {
        let operation = put_baseline(outcome, store, until, now)?;
        match store.compare_and_swap_head(&key, None, &operation, now)? {
            HeadUpdate::Applied(head) | HeadUpdate::Unchanged(head) => head,
            HeadUpdate::Conflict(_) => {
                let current = store
                    .read_head(&key, now)?
                    .ok_or("binding head disappeared")?;
                if current.bytes != bytes {
                    return Err("concurrent task baseline conflict".into());
                }
                current.identity
            }
        }
    };
    Ok(RegisteredPlan {
        outcome: outcome.clone(),
        head_key: key,
        head,
    })
}
/// 从私有原始快照重建 draft，再核对已存在的 active baseline，绝不创建或移动头。
/// 旧快照可保留审计，但迁移后不能用恢复入口取得当前 RegisteredPlan。
#[allow(clippy::too_many_arguments)]
pub fn restore_current(
    store: &EvidenceStore,
    operation: &str,
    now_ms: u64,
    project: &Path,
    contract: &CompiledContract,
    binding: &TaskBinding,
    subject: &CapturedSubject,
    budget: ExecutionBudget,
) -> Result<RegisteredPlan, String> {
    let outcome = load(
        store, operation, now_ms, project, contract, binding, subject, budget,
    )?;
    if outcome.plan.is_none() {
        return Err("incomplete stored draft".into());
    }
    let key = head_key(&outcome)?;
    let head = store
        .read_head(&key, now_ms)?
        .ok_or("active task binding missing")?;
    if head.bytes != baseline_bytes(&outcome)? {
        return Err("stored plan belongs to an obsolete task baseline".into());
    }
    Ok(RegisteredPlan {
        outcome,
        head_key: key,
        head: head.identity,
    })
}
