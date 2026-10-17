static SLOTS: OnceLock<Arc<tokio::sync::Semaphore>> = OnceLock::new();
use std::{
    sync::{Arc, OnceLock},
    time::Duration,
};
pub(crate) async fn run<T: Send + 'static>(
    action: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    run_until(std::time::Instant::now() + Duration::from_secs(30), action).await
}
pub(crate) async fn run_until<T: Send + 'static>(
    deadline: std::time::Instant,
    action: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    let deadline = tokio::time::Instant::from_std(
        deadline.min(std::time::Instant::now() + Duration::from_secs(30)),
    );
    let permit = tokio::time::timeout_at(
        deadline,
        SLOTS
            .get_or_init(|| Arc::new(tokio::sync::Semaphore::new(16)))
            .clone()
            .acquire_owned(),
    )
    .await
    .map_err(|_| "policy preparation queue timeout")?
    .map_err(|e| e.to_string())?;
    let task = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        action()
    });
    tokio::time::timeout_at(deadline, task)
        .await
        .map_err(|_| "policy preparation timeout")?
        .map_err(|e| e.to_string())?
}
