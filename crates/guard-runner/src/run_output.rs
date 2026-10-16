use crate::StopReason;
use guard_protocol::{ExecutionStatus, UtcTimestamp};
/// 有界私有原始输出，不实现反序列化，不携带 PASS 或可信标签。无 Java 对应类型。
#[derive(Debug)]
pub struct RunOutput {
    /// 仅为运行维度，不能映射成规则通过。
    pub status: ExecutionStatus,
    /// 实际观察的退出码；超时/取消可不存在。
    pub exit_code: Option<i32>,
    /// 停止原因，与 stderr 内容无关。
    pub reason: StopReason,
    /// 预算范围内保留的 stdout 原始字节，可能含敏感信息。
    pub stdout: Vec<u8>,
    /// 与 stdout 共享预算的私有 stderr 原始字节。
    pub stderr: Vec<u8>,
    /// 超过输出预算或排空超时；不得以部分输出声称完整覆盖。
    pub output_truncated: bool,
    /// 本次父进程PID，仅作诊断，不是跨重启身份。
    pub process_id: Option<u32>,
    /// 实际创建进程后的UTC观察；未派发保持None，不由外部报告倒推。
    pub started_at: Option<UtcTimestamp>,
    /// 受管树停止及输出排空后的UTC观察；不是可信授权时间。
    pub finished_at: Option<UtcTimestamp>,
}
impl RunOutput {
    pub(crate) fn stopped(reason: StopReason) -> Self {
        Self {
            status: match reason {
                StopReason::Timeout => ExecutionStatus::TimedOut,
                StopReason::Cancelled => ExecutionStatus::Cancelled,
                _ => ExecutionStatus::Failed,
            },
            exit_code: None,
            reason,
            stdout: Vec::new(),
            stderr: Vec::new(),
            output_truncated: false,
            process_id: None,
            started_at: None,
            finished_at: None,
        }
    }
}
