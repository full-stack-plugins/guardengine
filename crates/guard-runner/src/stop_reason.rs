/// 执行停止原因，与规则结论分离。无 Java 对应类型。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StopReason {
    /// 原生进程结束；退出0仍不代表质量通过。
    Exited,
    /// 总耗时预算用尽，含等待并发槽位。
    Timeout,
    /// 主动取消或调用 future 被丢弃。
    Cancelled,
    /// stdout/stderr 共享输出预算用尽。
    OutputLimit,
    /// 管道读取、进程查询或受管树终止异常。
    IoFailure,
    /// 执行后可执行文件身份漂移；保留已收集字节但不接受完整执行。
    ExecutableChanged,
    /// 墙钟无法表达或倒退；保留真实观察，禁止修饰成合法完成时间。
    ClockInvalid,
}
