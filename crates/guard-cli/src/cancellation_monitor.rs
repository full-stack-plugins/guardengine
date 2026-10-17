use crate::{
    cancel_request::CancelRequest, cli_failure::CliFailure, host_config::HostConfig, safe_files,
};
use guard_runner::Cancellation;
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};
/// 执行阶段的RAII取消观察器；错误请求停止进程但不伪装成成功取消。无Java对应类型。
pub(crate) struct CancellationMonitor {
    tasks: Vec<tokio::task::JoinHandle<()>>,
    failure: Arc<Mutex<Option<String>>>,
}
impl CancellationMonitor {
    /// 按宿主范围和可选请求文件启动取消监听，返回拥有任务的RAII句柄。
    pub(crate) fn start(
        cancel: Cancellation,
        host: &HostConfig,
        operation: &str,
        file: Option<&String>,
    ) -> Result<Self, CliFailure> {
        let failure = Arc::new(Mutex::new(None));
        let signal = cancel.clone();
        let mut tasks = Vec::new();
        if let Some(file) = file {
            let path = PathBuf::from(file);
            if !path.is_absolute() {
                return Err(CliFailure::Usage(
                    "absolute cancellation request path required".into(),
                ));
            }
            let parent = safe_files::plain(path.parent().ok_or("cancellation parent required")?)
                .map_err(CliFailure::Usage)?;
            if parent != safe_files::plain(&host.field::<PathBuf>("scratchRoot")?)? {
                return Err(CliFailure::Usage(
                    "cancel request must be direct child of configured scratchRoot".into(),
                ));
            }
            let expected = CancelRequest {
                schema_version: "guard.cli-cancel/v1".into(),
                repository_id: host.field("repositoryId")?,
                task_id: host.field("taskId")?,
                operation_id: guard_protocol::Identifier::new(operation)?,
            };
            prepare_file(&cancel, &path, &expected).map_err(CliFailure::Usage)?;
            let problem = failure.clone();
            tasks.push(tokio::spawn(async move {
                loop {
                    match inspect(&path, &expected) {
                        Ok(true) => {
                            cancel.cancel();
                            break;
                        }
                        Ok(false) => {}
                        Err(error) => {
                            if let Ok(mut slot) = problem.lock() {
                                *slot = Some(error.chars().take(512).collect());
                            }
                            cancel.cancel();
                            break;
                        }
                    }
                    tokio::time::sleep(Duration::from_millis(25)).await;
                }
            }));
        }
        tasks.push(tokio::spawn(async move {
            if tokio::signal::ctrl_c().await.is_ok() {
                signal.cancel();
            }
        }));
        Ok(Self { tasks, failure })
    }
    /// 返回非法取消请求的有界诊断，不把其视为成功取消。
    pub(crate) fn failure(&self) -> Option<String> {
        self.failure.lock().ok().and_then(|v| v.clone())
    }
}
impl Drop for CancellationMonitor {
    fn drop(&mut self) {
        for task in &self.tasks {
            task.abort();
        }
    }
}
fn inspect(path: &std::path::Path, expected: &CancelRequest) -> Result<bool, String> {
    match std::fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error.to_string()),
        Ok(_) => {}
    }
    let actual: CancelRequest = safe_files::decode(&safe_files::read(path, 4096)?)?;
    if &actual != expected {
        return Err("cancellation request scope/version/operation mismatch".into());
    }
    Ok(true)
}
fn prepare_file(
    cancel: &Cancellation,
    path: &std::path::Path,
    expected: &CancelRequest,
) -> Result<(), String> {
    if inspect(path, expected)? {
        cancel.cancel();
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    #[test]
    fn invalid_cancel_request_is_rejected_without_claiming_cancellation() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("cancel.json");
        let expected = crate::cancel_request::CancelRequest {
            schema_version: "guard.cli-cancel/v1".into(),
            repository_id: guard_protocol::Identifier::new("repo").unwrap(),
            task_id: guard_protocol::Identifier::new("task").unwrap(),
            operation_id: guard_protocol::Identifier::new("operation").unwrap(),
        };
        let cancel = guard_runner::Cancellation::new();
        assert!(super::prepare_file(&cancel, &path, &expected).is_ok());
        assert!(!cancel.is_cancelled());
        let mut value = serde_json::to_value(&expected).unwrap();
        value["operationId"] = serde_json::json!("different");
        std::fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
        assert!(super::prepare_file(&cancel, &path, &expected).is_err());
        assert!(!cancel.is_cancelled());
        value["operationId"] = serde_json::json!("operation");
        value["extra"] = serde_json::json!(true);
        std::fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
        assert!(super::prepare_file(&cancel, &path, &expected).is_err());
        std::fs::write(&path, vec![b' '; 4097]).unwrap();
        assert!(super::prepare_file(&cancel, &path, &expected).is_err());
        assert!(!cancel.is_cancelled());
    }
    #[test]
    fn existing_exact_request_cancels_before_an_async_task_can_be_scheduled() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("cancel.json");
        let expected = crate::cancel_request::CancelRequest {
            schema_version: "guard.cli-cancel/v1".into(),
            repository_id: guard_protocol::Identifier::new("repo").unwrap(),
            task_id: guard_protocol::Identifier::new("task").unwrap(),
            operation_id: guard_protocol::Identifier::new("operation").unwrap(),
        };
        std::fs::write(&path, serde_json::to_vec(&expected).unwrap()).unwrap();
        let cancel = guard_runner::Cancellation::new();
        super::prepare_file(&cancel, &path, &expected).unwrap();
        assert!(cancel.is_cancelled());
    }
}
