use crate::{file_input::safe_path, hash_file, snapshot_project, verify_receipt};
use serde_json::Value;
use std::{
    collections::BTreeSet,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

fn ids(value: &Value) -> Result<BTreeSet<&str>, String> {
    let values = value
        .as_array()
        .filter(|v| !v.is_empty() && v.len() <= 8192)
        .ok_or("nonempty bounded test IDs required")?;
    let mut result = BTreeSet::new();
    for id in values {
        let id = id
            .as_str()
            .filter(|s| !s.trim().is_empty() && s.len() <= 2048)
            .ok_or("invalid test ID")?;
        if !result.insert(id) {
            return Err("duplicate test ID".into());
        }
    }
    Ok(result)
}
/// 校验显式信任锚、执行声明、当前源码、计划、时效及产物；成功仍不构成审批。
/// 返回已核验 payload；受控生产者须保护密钥和运行环境，此函数不是沙箱。
pub fn verify_execution(
    envelope: &Value,
    public_key: &Path,
    project: &Path,
    expected_plan_digest: &str,
    max_age_seconds: u64,
) -> Result<Value, String> {
    let root = safe_path(project)?;
    if !root.is_dir() {
        return Err("project directory required".into());
    }
    if safe_path(public_key)?.starts_with(&root) {
        return Err("trust anchor must be outside the candidate project".into());
    }
    let payload = verify_receipt(envelope, public_key)?;
    if payload["schema_version"] != "testguard.execution/v1"
        || payload["producer"] != "testguard"
        || payload["project"].as_str() != root.to_str()
        || payload["status"] != "PASS"
        || payload["exit_code"].as_i64() != Some(0)
        || payload["execution_verified"] != true
        || !matches!(payload["runner"].as_str(), Some("maven" | "cargo"))
    {
        return Err(
            "execution producer, candidate or successful-process claim is inconsistent".into(),
        );
    }
    if !expected_plan_digest.starts_with("sha256:")
        || expected_plan_digest.len() != 71
        || payload["plan_digest"] != expected_plan_digest
    {
        return Err("execution plan differs from explicit obligation".into());
    }
    if max_age_seconds == 0 || max_age_seconds > 604800 {
        return Err("evidence maximum age must be 1..604800 seconds".into());
    }
    let now = u64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_millis(),
    )
    .map_err(|e| e.to_string())?;
    let start = payload["started_at_unix_ms"]
        .as_u64()
        .filter(|v| *v > 0)
        .ok_or("execution start time required")?;
    let finish = payload["finished_at_unix_ms"]
        .as_u64()
        .filter(|v| *v > 0)
        .ok_or("execution finish time required")?;
    if start > finish || finish > now || now - finish > max_age_seconds * 1000 {
        return Err("execution time is future, reversed or expired".into());
    }
    let required = ids(&payload["required_tests"])?;
    let observed = ids(&payload["observed_tests"])?;
    if !required.is_subset(&observed) {
        return Err("required tests missing from execution".into());
    }
    if payload["snapshot"] != snapshot_project(&root)? {
        return Err("current project differs from execution snapshot".into());
    }
    let argv = payload["argv"]
        .as_array()
        .filter(|a| !a.is_empty() && a.len() <= 128)
        .ok_or("bounded executed argv required")?;
    if argv
        .iter()
        .any(|a| a.as_str().is_none_or(|s| s.len() > 16384))
    {
        return Err("invalid execution argv".into());
    }
    let executable = Path::new(
        payload["executable"]["path"]
            .as_str()
            .ok_or("executed program path required")?,
    );
    if payload["executable"]["sha256"] != hash_file(executable)? {
        return Err("executed program bytes changed".into());
    }
    let artifacts = payload["artifacts"]
        .as_array()
        .filter(|a| !a.is_empty() && a.len() <= 1024)
        .ok_or("execution artifacts required")?;
    let mut paths = BTreeSet::new();
    for artifact in artifacts {
        let path = Path::new(artifact["path"].as_str().ok_or("artifact path required")?);
        if !paths.insert(safe_path(path)?) || artifact["sha256"] != hash_file(path)? {
            return Err("execution artifact duplicated or changed".into());
        }
    }
    Ok(payload)
}
