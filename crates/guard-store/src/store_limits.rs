/// 本地存储资源上限；不是可信时间或批准策略。无 Java 对应类型。
#[derive(Clone, Copy, Debug)]
pub struct StoreLimits {
    /// 单个私有原件的最大字节数。
    pub max_artifact_bytes: u64,
    /// 全部尚未删除原件的最大字节数，隔离原件也计入。
    pub max_total_bytes: u64,
    /// 元数据记录总上限；到达后拒绝新操作，不删除审计凑空间。
    pub max_records: u64,
    /// 相对于调用者提供时间的最长保留期限。
    pub max_retention_ms: u64,
    /// 写锁和 SQLite busy 的分别等待上限。
    pub busy_timeout_ms: u64,
}
impl Default for StoreLimits {
    fn default() -> Self {
        Self {
            max_artifact_bytes: 8 * 1024 * 1024,
            max_total_bytes: 128 * 1024 * 1024,
            max_records: 4096,
            max_retention_ms: 30 * 86400 * 1000,
            busy_timeout_ms: 300,
        }
    }
}
impl StoreLimits {
    pub(crate) fn validate(self) -> Result<(), String> {
        if self.max_artifact_bytes == 0
            || self.max_artifact_bytes > 128 * 1024 * 1024
            || self.max_total_bytes < self.max_artifact_bytes
            || self.max_total_bytes > 8 * 1024 * 1024 * 1024
            || self.max_records == 0
            || self.max_records > 65536
            || self.max_retention_ms == 0
            || self.max_retention_ms > 366 * 86400 * 1000
            || self.busy_timeout_ms == 0
            || self.busy_timeout_ms > 10000
        {
            Err("invalid bounded store limits".into())
        } else {
            Ok(())
        }
    }
}
