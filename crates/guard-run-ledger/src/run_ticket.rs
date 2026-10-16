use crate::run_record::RunRecord;
use guard_protocol::RunSequence;
use guard_store::HeadIdentity;
/// 单次受管运行票，不能由JSON重建；仅在同一私有registry使用。无Java对应类型。
#[derive(Clone, Debug)]
pub struct RunTicket {
    pub(crate) record: RunRecord,
    pub(crate) head: HeadIdentity,
    pub(crate) key: String,
}
impl RunTicket {
    /// 返回存储分配的运行序号，不接受客户端自报序号。
    pub fn sequence(&self) -> &RunSequence {
        &self.record.sequence
    }
}
