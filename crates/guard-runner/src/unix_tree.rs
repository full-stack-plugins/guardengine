use tokio::process::{Child, Command};
/// 协作式 Unix 进程组；恶意 setsid 逃逸需后续 OS 沙箱。无 Java 对应类型。
pub(crate) struct ProcessTree {
    group: Option<i32>,
}
impl ProcessTree {
    pub(crate) fn prepare(command: &mut Command) -> Result<Self, String> {
        command.process_group(0);
        Ok(Self { group: None })
    }
    pub(crate) fn attach(
        &mut self,
        child: &Child,
        _deadline: std::time::Instant,
    ) -> Result<bool, String> {
        self.group = Some(
            i32::try_from(child.id().ok_or("child missing before attachment")?)
                .map_err(|e| e.to_string())?,
        );
        Ok(true)
    }
    pub(crate) fn terminate(&self) -> Result<(), String> {
        if let Some(group) = self.group {
            // 负PID只指向本次新建的独立组；从不接受外部传入PID。
            let rc = unsafe { libc::kill(-group, libc::SIGKILL) };
            if rc != 0 && std::io::Error::last_os_error().raw_os_error() != Some(libc::ESRCH) {
                return Err(std::io::Error::last_os_error().to_string());
            }
        }
        Ok(())
    }
}
impl Drop for ProcessTree {
    fn drop(&mut self) {
        let _ = self.terminate();
    }
}
