use crate::{MigrationAuthority, MigrationOutcome, MigrationRequest, RegisteredPlan, freeze};
use guard_contract::CompiledContract;
use guard_protocol::{ExecutionBudget, TaskBinding};
use guard_store::{EvidenceStore, HeadUpdate};
use guard_subject::CapturedSubject;
use serde_json::json;
use std::path::Path;
/// 对 active binding 执行显式批准迁移；唯一允许不同契约替换已有任务头的领域入口。
/// 新原件先持久化、批准精确绑定旧/新快照和 revision，然后 CAS；竞争失败不移动头。
/// 缺权威、拒绝、错摘要、跨归属或陈旧头均拒绝，不签发任何 ExecutionGrant。
#[allow(clippy::too_many_arguments)]
pub fn migrate(
    previous: &RegisteredPlan,
    store: &EvidenceStore,
    retain_until_ms: u64,
    now_ms: u64,
    project: &Path,
    contract: &CompiledContract,
    binding: &TaskBinding,
    subject: &CapturedSubject,
    budget: ExecutionBudget,
    authority: Option<&dyn MigrationAuthority>,
) -> Result<MigrationOutcome, String> {
    let authority = authority.ok_or("independent migration approval authority required")?;
    previous.assert_binding_current(store, now_ms)?;
    if previous.outcome.binding.task_id != binding.task_id
        || previous.outcome.binding.scope.repository_ids != binding.scope.repository_ids
    {
        return Err("migration cannot switch task/repository identity".into());
    }
    let next = freeze(project, contract, binding, subject, budget)?;
    let Some(next_plan) = next.plan.as_ref() else {
        return Err("new baseline has unresolved target obligations".into());
    };
    if crate::registration::head_key(&next)? != previous.head_key {
        return Err("migration head identity changed".into());
    }
    let request_value = json!({"schemaVersion":"guard.plan-migration/v1", "previousHead":previous.head,
        "previousDigest":previous.outcome.digest,"nextDigest":next.digest,"previousBinding":previous.outcome.binding,"nextBinding":next.binding});
    let request_digest = guard_protocol::digest_json(
        &serde_json::to_vec(&request_value).map_err(|e| e.to_string())?,
    )?;
    let request = MigrationRequest {
        digest: request_digest,
        previous_digest: previous.outcome.digest.clone(),
        next_digest: next.digest.clone(),
        previous_binding: previous.outcome.binding.clone(),
        next_binding: next.binding.clone(),
        task_id: binding.task_id.clone(),
        previous_head: previous.head.clone(),
    };
    let approval = authority.verify(&request)?;
    if approval.request_digest != request.digest {
        return Err("migration approval refers to different revision or scope".into());
    }
    guard_subject::verify_current(project, &subject.binding, &subject.selection, subject)?;
    let operation = crate::registration::put_baseline(&next, store, retain_until_ms, now_ms)?;
    let audit = json!({"schemaVersion":"guard.approved-migration/v1","request":request_value,
        "requestDigest":request.digest,"authorityId":approval.authority_id,"approvalRef":approval.approval_ref});
    store.put(
        &format!("guard-plan:migration:{}", request.digest.as_str()),
        &guard_protocol::canonicalize(&serde_json::to_vec(&audit).map_err(|e| e.to_string())?)?,
        retain_until_ms,
        now_ms,
    )?;
    // Git核验与持久化可能耗时；CAS发布前重新读取独立批准，旧attestation不能永久复用。
    // 此边界检查不是与外部撤销源的分布式原子事务，动作消费仍须K10再次核验。
    previous.assert_binding_current(store, now_ms)?;
    let refreshed = authority.verify(&request)?;
    if refreshed.authority_id != approval.authority_id
        || refreshed.approval_ref != approval.approval_ref
        || refreshed.request_digest != approval.request_digest
    {
        return Err("migration approval identity changed before head publication".into());
    }
    let head = match store.compare_and_swap_head(
        &previous.head_key,
        Some(&previous.head),
        &operation,
        now_ms,
    )? {
        HeadUpdate::Applied(head) | HeadUpdate::Unchanged(head) => head,
        HeadUpdate::Conflict(_) => {
            return Err(
                "active task binding changed during migration approval; no head update".into(),
            );
        }
    };
    let revalidate = !next_plan.equivalent_inputs(previous.plan());
    Ok(MigrationOutcome {
        next: RegisteredPlan {
            outcome: next,
            head_key: previous.head_key.clone(),
            head,
        },
        previous_digest: previous.outcome.digest.clone(),
        approval,
        revalidate,
    })
}
