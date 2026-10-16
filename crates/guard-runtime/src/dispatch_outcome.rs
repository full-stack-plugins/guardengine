use crate::ExecutionObservation;
use guard_protocol::RunSequence;
/// 单飞调度结果；Existing不得再次运行。无Java对应类型。
#[derive(Debug)]
pub enum DispatchOutcome {
    Existing { sequence: RunSequence },
    Finished(Box<ExecutionObservation>),
}
