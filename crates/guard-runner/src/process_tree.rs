#[cfg(windows)]
#[path = "windows_tree.rs"]
mod implementation;
#[cfg(unix)]
#[path = "unix_tree.rs"]
mod implementation;
#[cfg(not(any(windows, unix)))]
compile_error!("guard-runner requires an explicitly supported process-tree backend");
pub(crate) use implementation::ProcessTree;

/// Windows额外确认Job已清空；Unix仍保持原先的协作式进程组终止语义。
pub(crate) async fn terminate_and_confirm(tree: &ProcessTree) -> Result<(), String> {
    tree.terminate()?;
    #[cfg(windows)]
    tree.wait_for_empty().await?;
    Ok(())
}
