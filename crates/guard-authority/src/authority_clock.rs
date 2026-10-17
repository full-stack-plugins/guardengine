use guard_protocol::UtcTimestamp;
/// 受保护宿主的核验时钟端口；不接受候选JSON传now。
pub trait AuthorityClock: Send + Sync {
    /// 返回本次核验实际时刻；回退由核验器保守拒绝。
    fn now(&self) -> Result<UtcTimestamp, String>;
}
