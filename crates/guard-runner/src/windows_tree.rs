use std::{
    mem::{size_of, zeroed},
    os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle},
    ptr::null,
    time::{Duration, Instant},
};
use tokio::process::{Child, Command};
use windows_sys::Win32::{
    Foundation::{ERROR_NO_MORE_FILES, GetLastError, HANDLE, INVALID_HANDLE_VALUE},
    System::{
        Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, TH32CS_SNAPTHREAD, THREADENTRY32, Thread32First, Thread32Next,
        },
        JobObjects::{
            AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
            JOBOBJECT_BASIC_ACCOUNTING_INFORMATION, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
            JobObjectBasicAccountingInformation, JobObjectExtendedLimitInformation,
            QueryInformationJobObject, SetInformationJobObject, TerminateJobObject,
        },
        Threading::{
            CREATE_NO_WINDOW, CREATE_SUSPENDED, OpenThread, ResumeThread, THREAD_SUSPEND_RESUME,
        },
    },
};
/// Windows Job 先接管暂停进程，再恢复主线程，关闭句柄终止整棵树。无 Java 对应类型。
pub(crate) struct ProcessTree {
    job: OwnedHandle,
}
impl ProcessTree {
    pub(crate) fn prepare(command: &mut Command) -> Result<Self, String> {
        // 新进程在任何用户代码执行前暂停，避免 spawn→assign 间子孙逃逸。
        command.creation_flags(CREATE_SUSPENDED | CREATE_NO_WINDOW);
        let job = owned(unsafe { CreateJobObjectW(null(), null()) })?;
        let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { zeroed() };
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        if unsafe {
            SetInformationJobObject(
                job.as_raw_handle(),
                JobObjectExtendedLimitInformation,
                (&limits as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
                size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
        } == 0
        {
            return Err(std::io::Error::last_os_error().to_string());
        }
        Ok(Self { job })
    }
    // false表示预算已耗尽且主线程仍暂停；Err表示接管/枚举本身失败。
    pub(crate) fn attach(&mut self, child: &Child, deadline: Instant) -> Result<bool, String> {
        let process_id = child.id().ok_or("suspended child missing")?;
        if unsafe {
            AssignProcessToJobObject(
                self.job.as_raw_handle(),
                child.raw_handle().ok_or("child handle missing")?,
            )
        } == 0
        {
            return Err(std::io::Error::last_os_error().to_string());
        }
        if Instant::now() >= deadline {
            return Ok(false);
        }
        // Tokio持有进程句柄但不公开主线程；暂停状态下枚举此新进程唯一线程。
        let snapshot = owned(unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) })?;
        let mut entry: THREADENTRY32 = unsafe { zeroed() };
        entry.dwSize = size_of::<THREADENTRY32>() as u32;
        let mut found = None;
        let started = Instant::now();
        let mut count = 0usize;
        let mut rc = unsafe { Thread32First(snapshot.as_raw_handle(), &mut entry) };
        while rc != 0 {
            count += 1;
            if Instant::now() >= deadline {
                return Ok(false);
            }
            if count > 65536 || started.elapsed() >= Duration::from_secs(5) {
                return Err("suspended thread inventory budget exceeded".into());
            }
            if entry.th32OwnerProcessID == process_id {
                if found.is_some() {
                    return Err("unexpected multiple suspended verifier threads".into());
                }
                found = Some(entry.th32ThreadID);
            }
            rc = unsafe { Thread32Next(snapshot.as_raw_handle(), &mut entry) };
        }
        if unsafe { GetLastError() } != ERROR_NO_MORE_FILES {
            return Err(std::io::Error::last_os_error().to_string());
        }
        let thread = owned(unsafe {
            OpenThread(
                THREAD_SUSPEND_RESUME,
                0,
                found.ok_or("suspended verifier thread missing")?,
            )
        })?;
        // 最后一次时钟核对必须紧邻ResumeThread；耗尽预算时保持暂停交由统一清理。
        if Instant::now() >= deadline {
            return Ok(false);
        }
        if unsafe { ResumeThread(thread.as_raw_handle()) } != 1 {
            return Err("verifier thread was not suspended exactly once".into());
        }
        Ok(true)
    }
    /// 查询真实Job记账信息；发出终止不等于全部进程对象已完成退出。
    pub(crate) async fn wait_for_empty(&self) -> Result<(), String> {
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                let mut accounting: JOBOBJECT_BASIC_ACCOUNTING_INFORMATION = unsafe { zeroed() };
                if unsafe {
                    QueryInformationJobObject(
                        self.job.as_raw_handle(),
                        JobObjectBasicAccountingInformation,
                        (&mut accounting as *mut JOBOBJECT_BASIC_ACCOUNTING_INFORMATION).cast(),
                        size_of::<JOBOBJECT_BASIC_ACCOUNTING_INFORMATION>() as u32,
                        std::ptr::null_mut(),
                    )
                } == 0
                {
                    return Err(std::io::Error::last_os_error().to_string());
                }
                if accounting.ActiveProcesses == 0 {
                    return Ok(());
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .map_err(|_| "Windows job cleanup timed out with active processes".to_string())?
    }

    pub(crate) fn terminate(&self) -> Result<(), String> {
        if unsafe { TerminateJobObject(self.job.as_raw_handle(), 130) } == 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
        Ok(())
    }
}
fn owned(handle: HANDLE) -> Result<OwnedHandle, String> {
    if handle.is_null() || handle == INVALID_HANDLE_VALUE {
        Err(std::io::Error::last_os_error().to_string())
    } else {
        Ok(unsafe { OwnedHandle::from_raw_handle(handle) })
    }
}

#[cfg(test)]
mod tests {
    use super::{ProcessTree, owned};
    use std::{os::windows::io::AsRawHandle, ptr::null_mut, time::Duration};
    use windows_sys::Win32::{Foundation::DuplicateHandle, System::Threading::GetCurrentProcess};

    #[tokio::test]
    async fn job_query_failure_is_an_error_not_an_empty_job() {
        let mut command = tokio::process::Command::new(std::env::current_exe().unwrap());
        let original = ProcessTree::prepare(&mut command).unwrap();
        let mut restricted = null_mut();
        // 复制真实Job句柄且不给QUERY权限，真实Win32调用必须失败，不能解释为零活动进程。
        assert_ne!(
            unsafe {
                DuplicateHandle(
                    GetCurrentProcess(),
                    original.job.as_raw_handle(),
                    GetCurrentProcess(),
                    &mut restricted,
                    0,
                    0,
                    0,
                )
            },
            0
        );
        let tree = ProcessTree {
            job: owned(restricted).unwrap(),
        };
        assert!(tree.wait_for_empty().await.is_err());
    }

    #[tokio::test]
    async fn real_live_job_times_out_then_termination_confirms_empty() {
        let mut command = tokio::process::Command::new(std::env::current_exe().unwrap());
        command
            .args([
                "--exact",
                "process_tree::implementation::tests::live_job_wait_worker",
                "--nocapture",
            ])
            .env("GUARD_RUNNER_JOB_WAIT_WORKER", "1")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .kill_on_drop(true);
        let mut tree = ProcessTree::prepare(&mut command).unwrap();
        let mut child = command.spawn().unwrap();
        assert!(
            tree.attach(&child, std::time::Instant::now() + Duration::from_secs(10))
                .unwrap()
        );
        let error = tree.wait_for_empty().await.unwrap_err();
        assert!(error.contains("timed out"), "{error}");
        assert!(
            child.try_wait().unwrap().is_none(),
            "nonempty Job must not be misreported as empty"
        );
        tree.terminate().unwrap();
        tree.wait_for_empty().await.unwrap();
        tokio::time::timeout(Duration::from_secs(2), child.wait())
            .await
            .unwrap()
            .unwrap();
    }

    #[tokio::test]
    async fn expired_attachment_keeps_native_user_code_suspended_until_cleanup() {
        let dir = tempfile::tempdir().unwrap();
        let marker = dir.path().join("user-code-ran");
        let mut command = tokio::process::Command::new(std::env::current_exe().unwrap());
        command
            .args([
                "--exact",
                "process_tree::implementation::tests::live_job_wait_worker",
                "--nocapture",
            ])
            .env("GUARD_RUNNER_JOB_WAIT_WORKER", "1")
            .env("GUARD_RUNNER_JOB_WAIT_MARKER", &marker)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .kill_on_drop(true);
        let mut tree = ProcessTree::prepare(&mut command).unwrap();
        let mut child = command.spawn().unwrap();
        assert!(
            !tree
                .attach(&child, std::time::Instant::now() - Duration::from_secs(1))
                .unwrap()
        );
        tokio::time::sleep(Duration::from_millis(300)).await;
        assert!(
            !marker.exists(),
            "expired attachment must not resume native user code"
        );
        assert!(child.try_wait().unwrap().is_none());
        tree.terminate().unwrap();
        tree.wait_for_empty().await.unwrap();
        child.wait().await.unwrap();
    }

    #[test]
    fn live_job_wait_worker() {
        if std::env::var_os("GUARD_RUNNER_JOB_WAIT_WORKER").is_some() {
            if let Some(marker) = std::env::var_os("GUARD_RUNNER_JOB_WAIT_MARKER") {
                std::fs::write(marker, b"native user code executed").unwrap();
            }
            std::thread::sleep(Duration::from_secs(60));
        }
    }
}
