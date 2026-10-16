use crate::run_record::RunRecord;
use crate::{
    FinalizeOutcome, RunIdentity, RunObservation, RunScope, RunState, RunTicket, StartOutcome,
};
use guard_protocol::{Identifier, RunSequence};
use guard_store::{EvidenceStore, HeadIdentity, HeadUpdate};
use serde::Serialize;
use serde_json::json;
use sha2::{Digest as _, Sha256};
/// 使用唯一受保护GuardStore的本地运行账本；不拥有进程或凭据。无Java对应类型。
pub struct RunLedger<'a> {
    store: &'a EvidenceStore,
}
fn canonical(value: &impl Serialize) -> Result<Vec<u8>, String> {
    guard_protocol::canonicalize(&serde_json::to_vec(value).map_err(|e| e.to_string())?)
}
fn name(prefix: &str, value: &impl Serialize) -> Result<String, String> {
    let digest = guard_protocol::digest_json(&canonical(value)?)?;
    Ok(format!("guard-run:{prefix}:{}", digest.as_str()))
}
impl<'a> RunLedger<'a> {
    /// 绑定受保护的唯一存储；不创建数据库或选择其他registry。
    pub fn new(store: &'a EvidenceStore) -> Self {
        Self { store }
    }
    fn key(scope: &RunScope) -> Result<String, String> {
        name(
            "head",
            &json!({"repository":scope.repository_id,"task":scope.task_id,"obligation":scope.obligation_id}),
        )
    }
    fn current(
        &self,
        scope: &RunScope,
        now: u64,
    ) -> Result<Option<(HeadIdentity, RunRecord)>, String> {
        let Some(head) = self.store.read_head(&Self::key(scope)?, now)? else {
            return Ok(None);
        };
        // 严格JSON入口拒绝重复键；不从调用方JSON重建运行票。
        let bytes = guard_protocol::canonicalize(&head.bytes)?;
        let record: RunRecord = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        if record.schema_version != "guard.run-record/v1"
            || &record.scope != scope
            || record.sequence.as_str() == "0"
            || record.state == RunState::Stale
        {
            return Err(
                "run head scope/version/state mismatch; explicit scope migration required".into(),
            );
        }
        match (&record.result_operation, &record.result_digest) {
            (Some(op), Some(digest)) if record.state.terminal() => {
                self.store.read_private(op, now)?;
                if self.store.summary(op)?.content_digest != *digest {
                    return Err("result digest mismatch".into());
                }
            }
            (None, None) if matches!(record.state, RunState::Running | RunState::Incomplete) => {}
            _ => return Err("invalid run result references".into()),
        }
        Ok(Some((head.identity, record)))
    }
    fn save(&self, record: &RunRecord, now: u64) -> Result<String, String> {
        let op = name("record", record)?;
        self.store
            .put(&op, &canonical(record)?, record.retain_until_ms, now)?;
        Ok(op)
    }
    /// 原子取得单飞运行；序号由持久旧头递增，只有Started允许派发新进程。
    /// operation_id为宿主幂等键；不得为同一任务重复使用已消耗的键。
    pub fn start(
        &self,
        scope: &RunScope,
        identity: &RunIdentity,
        operation_id: &str,
        retain_until_ms: u64,
        now_ms: u64,
    ) -> Result<StartOutcome, String> {
        let operation_id = Identifier::new(operation_id)?;
        let key = Self::key(scope)?;
        let current = self.current(scope, now_ms)?;
        if let Some((head, record)) = &current {
            if record.state == RunState::Running {
                if record.operation_id == operation_id
                    && &record.identity == identity
                    && record.retain_until_ms == retain_until_ms
                {
                    return Ok(StartOutcome::Existing(RunTicket {
                        record: record.clone(),
                        head: head.clone(),
                        key,
                    }));
                }
                return Err(
                    "obligation already running; explicit recovery/cancellation required".into(),
                );
            }
            if record.operation_id == operation_id {
                return Err("operation already terminal; use a fresh operation ID".into());
            }
        }
        let sequence = match &current {
            Some((_, r)) => r
                .sequence
                .as_str()
                .parse::<u64>()
                .map_err(|e| e.to_string())?
                .checked_add(1)
                .ok_or("run sequence exhausted")?,
            None => 1,
        };
        let record = RunRecord {
            schema_version: "guard.run-record/v1".into(),
            scope: scope.clone(),
            identity: identity.clone(),
            sequence: RunSequence::new(sequence.to_string())?,
            operation_id,
            state: RunState::Running,
            retain_until_ms,
            result_operation: None,
            result_digest: None,
        };
        // 永久消耗operation ID；CAS失败可留下非active意图，但不可用旧ID重启另一轮。
        let claim = name(
            "operation",
            &json!({"key":key,"operation":record.operation_id}),
        )?;
        self.store
            .put(&claim, &canonical(&record)?, retain_until_ms, now_ms)?;
        let op = self.save(&record, now_ms)?;
        let expected = current.as_ref().map(|(h, _)| h);
        match self
            .store
            .compare_and_swap_head(&key, expected, &op, now_ms)?
        {
            HeadUpdate::Applied(head) => Ok(StartOutcome::Started(RunTicket { record, head, key })),
            HeadUpdate::Unchanged(head) => {
                Ok(StartOutcome::Existing(RunTicket { record, head, key }))
            }
            HeadUpdate::Conflict(_) => {
                Err("concurrent start lost; no process may be dispatched".into())
            }
        }
    }
    /// 先保存本地原始结果；仅精确运行票可终结当前running头，迟到结果只作私有审计。
    pub fn finalize(
        &self,
        ticket: &RunTicket,
        state: RunState,
        raw_result: &[u8],
        now_ms: u64,
    ) -> Result<FinalizeOutcome, String> {
        if !state.terminal() {
            return Err("final state must be terminal".into());
        }
        if raw_result.len() > 4 * 1024 * 1024 {
            return Err("result exceeds local bound".into());
        }
        let raw_op = name(
            "result",
            // 原始日志是二进制原件，不能转JSON数组后受协议数组/文档预算误拒。
            &json!({"ticket":ticket.record,"state":state,"rawDigest":format!("sha256:{:x}", Sha256::digest(raw_result))}),
        )?;
        let summary = self
            .store
            .put(&raw_op, raw_result, ticket.record.retain_until_ms, now_ms)?;
        let mut final_record = ticket.record.clone();
        final_record.state = state;
        final_record.result_operation = Some(raw_op);
        final_record.result_digest = Some(summary.content_digest);
        // 保存完整lineage，即便CAS失败也保留迟到结果；原件存在本身不赋予active资格。
        let op = self.save(&final_record, now_ms)?;
        let current = self
            .current(&ticket.record.scope, now_ms)?
            .ok_or("run head missing")?;
        if current.1 == final_record {
            return Ok(FinalizeOutcome::Unchanged);
        }
        if current.0 != ticket.head || current.1 != ticket.record {
            return Ok(FinalizeOutcome::AuditOnly);
        }
        match self
            .store
            .compare_and_swap_head(&ticket.key, Some(&ticket.head), &op, now_ms)?
        {
            HeadUpdate::Applied(_) => Ok(FinalizeOutcome::Applied),
            HeadUpdate::Unchanged(_) => Ok(FinalizeOutcome::Unchanged),
            HeadUpdate::Conflict(_) => Ok(FinalizeOutcome::AuditOnly),
        }
    }
    /// 只对完整相同输入提供状态；任一身份变化显示Stale，不复用旧PASS。
    pub fn inspect(
        &self,
        scope: &RunScope,
        identity: &RunIdentity,
        now_ms: u64,
    ) -> Result<Option<RunObservation>, String> {
        Ok(self.current(scope, now_ms)?.map(|(_, r)| {
            let same = &r.identity == identity;
            RunObservation {
                sequence: r.sequence,
                state: if same { r.state } else { RunState::Stale },
                result_digest: if same { r.result_digest } else { None },
                trust: "local_unverified",
            }
        }))
    }
    /// 立即撤销所见running资格，不证明OS进程死亡；不重放、不查PID、不杀进程。
    /// 控制端恢复调度前必须独立停止/隔离旧执行树；并发新头不会被覆盖。
    pub fn recover(&self, scope: &RunScope, now_ms: u64) -> Result<bool, String> {
        let Some((head, mut record)) = self.current(scope, now_ms)? else {
            return Ok(false);
        };
        if record.state != RunState::Running {
            return Ok(false);
        }
        record.state = RunState::Incomplete;
        let op = self.save(&record, now_ms)?;
        match self
            .store
            .compare_and_swap_head(&Self::key(scope)?, Some(&head), &op, now_ms)?
        {
            HeadUpdate::Applied(_) => Ok(true),
            HeadUpdate::Unchanged(_) | HeadUpdate::Conflict(_) => Ok(false),
        }
    }
}
