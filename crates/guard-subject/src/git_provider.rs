use guard_protocol::Digest;
use sha2::{Digest as ShaDigest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    io::Read,
    path::PathBuf,
    sync::OnceLock,
    time::{Duration, Instant},
};

static PROVIDER: OnceLock<Option<GitProvider>> = OnceLock::new();

/// 由宿主显式固定的 Git 可执行文件、原件摘要与环境；配置本身不产生授信。无 Java 对应类型。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitProvider {
    /// 原生 Git 可执行文件的绝对路径，路径及其祖先不得为链接。
    pub executable: PathBuf,
    /// 宿主独立提供的可执行文件 SHA256 原件摘要。
    pub artifact_digest: Digest,
    /// 清空继承环境后唯一保留的宿主环境；GIT_* 由库独占设置。
    pub environment: BTreeMap<String, String>,
}

/// 校验并冻结进程级 Git 配置；完全相同的重复配置幂等，默认 SDK 已使用后禁止晚切换。
/// 参数须由宿主明确提供；返回校验/不可变性错误，不修改进程全局环境，也不授予信任。
pub fn configure_git_provider(provider: GitProvider) -> Result<(), String> {
    provider.validate_environment()?;
    provider.verify_artifact(Instant::now() + Duration::from_secs(60))?;
    let current = PROVIDER.get_or_init(|| Some(provider.clone()));
    if current.as_ref() == Some(&provider) {
        Ok(())
    } else {
        Err("Git provider already frozen with a different configuration or SDK default".into())
    }
}

pub(crate) fn selected() -> Option<&'static GitProvider> {
    // 即使首次默认 Git 命令失败，也不能在稍后改变已启动采集的工具来源。
    PROVIDER.get_or_init(|| None).as_ref()
}

impl GitProvider {
    fn validate_environment(&self) -> Result<(), String> {
        if self.environment.len() > 256 {
            return Err("Git provider environment entry limit exceeded".into());
        }
        let mut names = BTreeSet::new();
        let mut total = 0usize;
        for (name, value) in &self.environment {
            let mut bytes = name.bytes();
            let first = bytes
                .next()
                .ok_or("Git provider environment key is empty")?;
            if !(first.is_ascii_alphabetic() || first == b'_')
                || !bytes.all(|b| b.is_ascii_alphanumeric() || b == b'_')
                || name.len() > 128
                || value.contains('\0')
                || value.len() > 32768
            {
                return Err("Git provider environment key/value is invalid or oversized".into());
            }
            let folded = name.to_ascii_uppercase();
            if folded.starts_with("GIT_") || !names.insert(folded) {
                return Err("Git provider environment GIT_* or case alias is forbidden".into());
            }
            total += name.len() + value.len();
            if total > 128 * 1024 {
                return Err("Git provider environment byte limit exceeded".into());
            }
        }
        Ok(())
    }

    pub(crate) fn verify_artifact(&self, deadline: Instant) -> Result<(), String> {
        crate::paths::safe_existing(&self.executable)?;
        let metadata = fs::metadata(&self.executable).map_err(|e| e.to_string())?;
        if !metadata.is_file() || metadata.len() > 512 * 1024 * 1024 {
            return Err("Git provider executable must be a bounded regular file".into());
        }
        #[cfg(windows)]
        if !self
            .executable
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("exe"))
        {
            return Err("Git provider executable must be a native .exe".into());
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if metadata.permissions().mode() & 0o111 == 0 {
                return Err("Git provider file is not executable".into());
            }
        }
        let mut file = File::open(&self.executable).map_err(|e| e.to_string())?;
        let mut hash = Sha256::new();
        let mut bytes = [0u8; 65536];
        let mut size = 0u64;
        loop {
            if Instant::now() >= deadline {
                return Err("Git capture deadline exceeded during executable verification".into());
            }
            let count = file.read(&mut bytes).map_err(|e| e.to_string())?;
            if count == 0 {
                break;
            }
            size += count as u64;
            if size > 512 * 1024 * 1024 {
                return Err("Git provider executable size limit exceeded".into());
            }
            hash.update(&bytes[..count]);
        }
        let actual = format!("sha256:{:x}", hash.finalize());
        // 前后再次校验路径，拒绝当前可观察到的链接替换；不宣称提供 OS 原子隔离。
        crate::paths::safe_existing(&self.executable)?;
        if Instant::now() >= deadline {
            return Err("Git capture deadline exceeded during executable verification".into());
        }
        if size != metadata.len() || actual != self.artifact_digest.as_str() {
            return Err("Git provider executable digest mismatch".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::GitProvider;
    use guard_protocol::Digest;
    use sha2::{Digest as ShaDigest, Sha256};
    use std::{
        collections::BTreeMap,
        fs,
        time::{Duration, Instant},
    };

    #[test]
    fn executable_hashing_respects_the_existing_deadline() {
        let temp = tempfile::tempdir().unwrap();
        let executable = temp.path().join("bounded.exe");
        fs::write(
            &executable,
            b"controlled fixture, not an actual Git executable",
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
        }
        let provider = GitProvider {
            artifact_digest: Digest::new(format!(
                "sha256:{:x}",
                Sha256::digest(fs::read(&executable).unwrap())
            ))
            .unwrap(),
            executable,
            environment: BTreeMap::new(),
        };
        assert!(
            provider
                .verify_artifact(Instant::now() - Duration::from_millis(1))
                .unwrap_err()
                .contains("deadline")
        );
    }

    #[cfg(unix)]
    #[test]
    fn executable_symlink_and_symlink_ancestor_are_rejected() {
        use std::os::unix::fs::{PermissionsExt, symlink};
        let temp = tempfile::tempdir().unwrap();
        let actual = temp.path().join("actual");
        fs::create_dir(&actual).unwrap();
        let executable = actual.join("git");
        fs::write(&executable, b"controlled fixture").unwrap();
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
        symlink(&actual, temp.path().join("linked_dir")).unwrap();
        symlink(&executable, temp.path().join("linked_git")).unwrap();
        for path in [
            temp.path().join("linked_git"),
            temp.path().join("linked_dir/git"),
        ] {
            let p = GitProvider {
                executable: path,
                artifact_digest: Digest::new(format!(
                    "sha256:{:x}",
                    Sha256::digest(b"controlled fixture")
                ))
                .unwrap(),
                environment: BTreeMap::new(),
            };
            assert!(
                p.verify_artifact(Instant::now() + Duration::from_secs(1))
                    .unwrap_err()
                    .contains("symlink")
            );
        }
    }
}

#[cfg(all(test, windows))]
mod windows_tests {
    use super::GitProvider;
    use guard_protocol::Digest;
    use sha2::{Digest as ShaDigest, Sha256};
    use std::{
        collections::BTreeMap,
        fs,
        path::PathBuf,
        process::Command,
        time::{Duration, Instant},
    };

    #[test]
    fn executable_junction_ancestor_is_rejected() {
        let temp = tempfile::tempdir().unwrap();
        let actual = temp.path().join("actual");
        fs::create_dir(&actual).unwrap();
        fs::write(actual.join("git.exe"), b"controlled fixture").unwrap();
        let linked = temp.path().join("linked");
        let command = PathBuf::from(std::env::var_os("SystemRoot").unwrap())
            .join("System32/WindowsPowerShell/v1.0/powershell.exe");
        let output = Command::new(command)
            .args(["-NoProfile", "-NonInteractive", "-Command",
                "$ErrorActionPreference='Stop'; New-Item -ItemType Junction -Path $env:SUBJECT_LINK -Target $env:SUBJECT_TARGET | Out-Null"])
            .env("SUBJECT_LINK", linked.to_string_lossy().replace('/', "\\").replace(r"\\?\", ""))
            .env("SUBJECT_TARGET", actual.to_string_lossy().replace('/', "\\").replace(r"\\?\", ""))
            .output().unwrap();
        assert!(
            output.status.success(),
            "junction fixture creation failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let p = GitProvider {
            executable: linked.join("git.exe"),
            artifact_digest: Digest::new(format!(
                "sha256:{:x}",
                Sha256::digest(b"controlled fixture")
            ))
            .unwrap(),
            environment: BTreeMap::new(),
        };
        let error = p
            .verify_artifact(Instant::now() + Duration::from_secs(1))
            .unwrap_err();
        assert!(
            error.contains("reparse") || error.contains("symlink"),
            "{error}"
        );
    }
}
