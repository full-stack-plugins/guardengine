use fs2::FileExt;
use std::{
    fs::File,
    path::Path,
    thread,
    time::{Duration, Instant},
};
/// 跨进程协作写锁；文件句柄存活至操作与恢复结束。无 Java 对应类型。
pub(crate) struct StoreLock(File);
impl StoreLock {
    pub(crate) fn acquire(path: &Path, wait_ms: u64) -> Result<Self, String> {
        let file = crate::filesystem::open(path, false, true)?;
        let start = Instant::now();
        loop {
            match file.try_lock_exclusive() {
                Ok(()) => return Ok(Self(file)),
                Err(error)
                    if error.kind() == std::io::ErrorKind::WouldBlock
                        || error.raw_os_error() == fs2::lock_contended_error().raw_os_error() =>
                {
                    if start.elapsed() >= Duration::from_millis(wait_ms) {
                        return Err("store busy".into());
                    }
                    thread::sleep(Duration::from_millis(5));
                }
                Err(error) => return Err(error.to_string()),
            }
        }
    }
}
impl Drop for StoreLock {
    fn drop(&mut self) {
        let _ = FileExt::unlock(&self.0);
    }
}
