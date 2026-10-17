use crate::{host_config::HostConfig, safe_files::get};
/// 在建立异步线程池前冻结实际Git提供者，不修改全局PATH或环境。
pub(crate) fn configure(host: &HostConfig) -> Result<(), String> {
    let value = host
        .data
        .get("git")
        .ok_or("fixed host Git provider required")?;
    crate::file_ports::closed(value, &["executable", "artifactDigest", "environment"])?;
    guard_subject::configure_git_provider(guard_subject::GitProvider {
        executable: get(value, "executable")?,
        artifact_digest: get(value, "artifactDigest")?,
        environment: get(value, "environment")?,
    })
}
