use crate::safe_files;
use serde_json::Value;
use std::path::PathBuf;
/// 固定宿主入口的有界配置；位置不证明OS保护，不读取候选自带信任配置。无Java对应类型。
#[derive(Clone)]
pub(crate) struct HostConfig {
    pub root: PathBuf,
    pub data: Value,
    digest: guard_protocol::Digest,
}
impl HostConfig {
    /// 从固定系统入口加载有界宿主配置，返回已检查路径与身份。
    pub(crate) fn load() -> Result<Self, String> {
        let root = safe_files::plain(&host_root()?)?;
        let bytes = safe_files::read(&root.join("host.json"), 1024 * 1024)?;
        let digest = guard_protocol::digest_json(&bytes)?;
        let data: Value = safe_files::decode(&bytes)?;
        let object = data.as_object().ok_or("host object required")?;
        let allowed = [
            "schemaVersion",
            "project",
            "repositoryId",
            "taskId",
            "actor",
            "audience",
            "worktreeId",
            "action",
            "authorizationRefs",
            "taskBinding",
            "layers",
            "verifiers",
            "policy",
            "stateRoot",
            "scratchRoot",
            "retentionMs",
            "budget",
            "anchors",
            "receipts",
            "revocations",
            "environmentDigest",
            "producers",
            "git",
        ];
        if data["schemaVersion"] != "guard.cli-host/v1"
            || object.keys().any(|k| !allowed.contains(&k.as_str()))
        {
            return Err("unsupported/unknown host configuration".into());
        }
        let result = Self { root, data, digest };
        let project = result.project()?;
        if result.root.starts_with(&project) || project.starts_with(&result.root) {
            return Err("host configuration and candidate must be disjoint".into());
        }
        let mut roots = Vec::new();
        for key in ["scratchRoot", "stateRoot"] {
            let path: PathBuf = result.field(key)?;
            if !path.is_absolute() {
                return Err("absolute private root required".into());
            }
            let existing = if path.exists() {
                safe_files::plain(&path)?
            } else {
                safe_files::plain(path.parent().ok_or("state parent required")?)?
                    .join(path.file_name().ok_or("state name required")?)
            };
            if existing.starts_with(&project)
                || project.starts_with(&existing)
                || existing.starts_with(&result.root)
                || result.root.starts_with(&existing)
            {
                return Err("private roots must be outside candidate and trust config".into());
            }
            roots.push(existing);
        }
        if roots[0].starts_with(&roots[1]) || roots[1].starts_with(&roots[0]) {
            return Err("stateRoot and scratchRoot must be disjoint".into());
        }
        Ok(result)
    }
    /// 按字段名读取宿主类型值，返回缺失或解析错误。
    pub(crate) fn field<T: serde::de::DeserializeOwned>(&self, key: &str) -> Result<T, String> {
        safe_files::get(&self.data, key)
    }
    /// 重读固定入口并核对初始配置摘要，返回同代配置。
    pub(crate) fn reload_same_profile(&self) -> Result<Self, String> {
        let current = Self::load()?;
        if current.root != self.root || current.digest != self.digest {
            return Err(
                "host profile changed during operation; explicit new invocation required".into(),
            );
        }
        Ok(current)
    }
    /// 返回既有普通项目目录，拒绝链接与不合法路径。
    pub(crate) fn project(&self) -> Result<PathBuf, String> {
        let path = safe_files::plain(&self.field::<PathBuf>("project")?)?;
        if !path.is_dir() {
            return Err("existing project directory required".into());
        }
        Ok(path)
    }
    /// 读取宿主根内相对文件，返回有界原始字节。
    pub(crate) fn file(&self, name: &str) -> Result<Vec<u8>, String> {
        safe_files::read(&safe_files::relative(&self.root, name)?, 1024 * 1024)
    }
    /// 返回宿主明确引用的普通政策包目录。
    pub(crate) fn policy_root(&self) -> Result<PathBuf, String> {
        safe_files::relative(&self.root, &self.field::<String>("policy")?)
    }
    /// 读取正式协议原件，返回真实任务绑定而非执行资格。
    pub(crate) fn binding(&self) -> Result<guard_protocol::TaskBinding, String> {
        let bytes = self.file(&self.field::<String>("taskBinding")?)?;
        match guard_protocol::decode(&bytes)?.document() {
            guard_protocol::ProtocolDocument::TaskBinding(v) => Ok(v.clone()),
            _ => Err("TaskBinding protocol document required".into()),
        }
    }
}
#[cfg(feature = "test-fixture")]
fn host_root() -> Result<PathBuf, String> {
    Ok(option_env!("GUARDCORE_TEST_HOST_ROOT")
        .ok_or("test-fixture needs compile-time GUARDCORE_TEST_HOST_ROOT")?
        .into())
}
#[cfg(all(not(feature = "test-fixture"), not(windows)))]
fn host_root() -> Result<PathBuf, String> {
    Ok("/etc/guardcore".into())
}
#[cfg(all(not(feature = "test-fixture"), windows))]
fn host_root() -> Result<PathBuf, String> {
    use std::os::windows::ffi::OsStringExt;
    use windows_sys::Win32::{
        System::Com::CoTaskMemFree,
        UI::Shell::{FOLDERID_ProgramData, SHGetKnownFolderPath},
    };
    // 系统KnownFolder不采用调用方可替换的ProgramData环境变量。
    unsafe {
        let mut pointer = std::ptr::null_mut();
        if SHGetKnownFolderPath(&FOLDERID_ProgramData, 0, std::ptr::null_mut(), &mut pointer) < 0
            || pointer.is_null()
        {
            return Err("system ProgramData unavailable".into());
        }
        let mut len = 0;
        while len < 32768 && *pointer.add(len) != 0 {
            len += 1;
        }
        let value = if len == 32768 {
            Err("system folder exceeds bound".into())
        } else {
            Ok(
                PathBuf::from(std::ffi::OsString::from_wide(std::slice::from_raw_parts(
                    pointer, len,
                )))
                .join("GuardCore"),
            )
        };
        CoTaskMemFree(pointer.cast());
        value
    }
}
