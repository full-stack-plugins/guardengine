use crate::run_record::RunRecord;
use crate::{RunIdentity, RunScope, RunState};
use guard_protocol::{Digest, Identifier, RunSequence};
use guard_store::HeadIdentity;
/// 当前本地账本的只读完整快照；公开start/finalize也能制造它，绝非真实派发来源资格。无Java对应类型。
#[derive(Clone, Debug)]
pub struct CurrentRunReceipt {
    pub(crate) head: HeadIdentity,
    pub(crate) record: RunRecord,
}
impl CurrentRunReceipt {
    /// 返回完整CAS头身份，不能只用序号代替。
    pub fn head(&self) -> &HeadIdentity {
        &self.head
    }
    /// 返回稳定任务/义务范围。
    pub fn scope(&self) -> &RunScope {
        &self.record.scope
    }
    /// 返回全部冻结输入身份声明。
    pub fn identity(&self) -> &RunIdentity {
        &self.record.identity
    }
    /// 返回本轮操作ID。
    pub fn operation_id(&self) -> &Identifier {
        &self.record.operation_id
    }
    /// 返回存储分配序号。
    pub fn sequence(&self) -> &RunSequence {
        &self.record.sequence
    }
    /// 返回原本终态，不转换为质量判断。
    pub fn state(&self) -> RunState {
        self.record.state
    }
    /// 返回终态metadata原件操作名；构造时已检查存在。
    pub fn metadata_operation(&self) -> &str {
        self.record
            .result_operation
            .as_deref()
            .expect("checked terminal metadata")
    }
    /// 返回终态metadata原始字节摘要。
    pub fn metadata_digest(&self) -> &Digest {
        self.record
            .result_digest
            .as_ref()
            .expect("checked terminal metadata")
    }
}
