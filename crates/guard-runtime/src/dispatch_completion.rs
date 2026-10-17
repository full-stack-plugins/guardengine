use crate::DispatchProvenance;
use guard_plan::RegisteredPlan;
use guard_run_ledger::{CurrentRunReceipt, RunLedger};
use guard_store::EvidenceStore;
use std::{path::PathBuf, sync::Arc};
/// 仅真实Runtime派发且Applied终结路径可铸造的来源凭据；不代表规则通过。无Java对应类型。
/// ```compile_fail
/// let _: guard_runtime::DispatchCompletion = serde_json::from_str("{}").unwrap();
/// ```
/// ```compile_fail
/// let _ = guard_runtime::DispatchCompletion {};
/// ```
pub struct DispatchCompletion {
    pub(crate) plan: RegisteredPlan,
    pub(crate) store: Arc<EvidenceStore>,
    pub(crate) project: PathBuf,
    pub(crate) current: CurrentRunReceipt,
    pub(crate) metadata: Vec<u8>,
    pub(crate) provenance: DispatchProvenance,
}
impl DispatchCompletion {
    /// 返回只读机械声明；复制/解析声明不能构造本凭据。
    pub fn provenance(&self) -> &DispatchProvenance {
        &self.provenance
    }
    /// 消费前重读当前计划/运行头/原件；不宣称与外部撤销或未来动作原子化。
    pub fn verify_current(&self, now_ms: u64) -> Result<(), String> {
        self.plan
            .assert_current(&self.store, &self.project, now_ms)?;
        let ledger = RunLedger::new(&self.store);
        ledger.assert_current_receipt(&self.current, now_ms)?;
        if self
            .store
            .read_private(self.current.metadata_operation(), now_ms)?
            != self.metadata
        {
            return Err("runtime metadata differs from sealed dispatch bytes".into());
        }
        for artifact in [
            &self.provenance.metadata,
            &self.provenance.stdout,
            &self.provenance.stderr,
        ] {
            let bytes = self.store.read_private(&artifact.operation, now_ms)?;
            let summary = self.store.summary(&artifact.operation)?;
            if summary.content_digest != artifact.digest
                || summary.size != artifact.size
                || bytes.len() as u64 != artifact.size
            {
                return Err("dispatch raw artifact identity mismatch".into());
            }
        }
        ledger.assert_current_receipt(&self.current, now_ms)?;
        self.plan.assert_current(&self.store, &self.project, now_ms)
    }
}

impl std::fmt::Debug for DispatchCompletion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DispatchCompletion")
            .field("sequence", &self.provenance.sequence)
            .finish_non_exhaustive()
    }
}
