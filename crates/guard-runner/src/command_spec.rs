use guard_protocol::Digest;
use sha2::{Digest as _, Sha256};
use std::{
    collections::BTreeMap,
    io::Read,
    path::{Path, PathBuf},
};
/// 宿主明确选择的原生验证器 argv；不是从报告/候选命令文本提取。无 Java 对应类型。
#[derive(Clone, Debug)]
pub struct CommandSpec {
    /// 绝对原生可执行文件；Windows 不支持 cmd/bat 等隐式 shell。
    pub executable: PathBuf,
    /// 实际文件 SHA256 pin；调用前、执行后重新核对。
    pub executable_digest: Digest,
    /// 独立传递的字面参数；不进行 shell 展开。
    pub args: Vec<String>,
    /// 实际存在的绝对工作目录。
    pub directory: PathBuf,
    /// 由控制端明确提供的全部环境；不会继承宿主凭据。
    pub environment: BTreeMap<String, String>,
}
impl CommandSpec {
    pub(crate) fn validate(&self) -> Result<(), String> {
        if !self.executable.is_absolute()
            || !self.directory.is_absolute()
            || !self.directory.is_dir()
            || self.args.len() > 1024
            || self
                .args
                .iter()
                .any(|a| a.contains('\0') || a.len() > 32768)
            || self.args.iter().map(String::len).sum::<usize>() > 128 * 1024
            || self.environment.len() > 256
            || self.environment.iter().any(|(k, v)| {
                k.is_empty()
                    || k.contains(['\0', '='])
                    || v.contains('\0')
                    || k.len() + v.len() > 32768
            })
            || self
                .environment
                .iter()
                .map(|(k, v)| k.len() + v.len())
                .sum::<usize>()
                > 128 * 1024
        {
            return Err("invalid bounded argv, directory or environment".into());
        }
        #[cfg(windows)]
        if !self
            .executable
            .extension()
            .is_some_and(|v| v.eq_ignore_ascii_case("exe"))
        {
            return Err(
                "Windows verifier must be a pinned native exe, not an implicit shell".into(),
            );
        }
        self.verify_executable()
    }
    pub(crate) fn verify_executable(&self) -> Result<(), String> {
        reject_links(&self.executable)?;
        let metadata = std::fs::symlink_metadata(&self.executable).map_err(|e| e.to_string())?;
        if !metadata.is_file() || metadata.len() > 512 * 1024 * 1024 {
            return Err("bounded regular executable required".into());
        }
        let mut file = std::fs::File::open(&self.executable).map_err(|e| e.to_string())?;
        let mut hash = Sha256::new();
        let mut buffer = [0; 65536];
        let mut total = 0_u64;
        loop {
            let len = file.read(&mut buffer).map_err(|e| e.to_string())?;
            if len == 0 {
                break;
            }
            total += len as u64;
            if total > 512 * 1024 * 1024 {
                return Err("executable changed beyond budget".into());
            }
            hash.update(&buffer[..len]);
        }
        if total != metadata.len()
            || format!("sha256:{:x}", hash.finalize()) != self.executable_digest.as_str()
        {
            return Err("executable pin mismatch".into());
        }
        Ok(())
    }
}
fn reject_links(path: &Path) -> Result<(), String> {
    for ancestor in path.ancestors() {
        let meta = std::fs::symlink_metadata(ancestor).map_err(|e| e.to_string())?;
        if meta.file_type().is_symlink() {
            return Err("linked verifier path rejected".into());
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            if meta.file_attributes() & 0x400 != 0 {
                return Err("reparse verifier path rejected".into());
            }
        }
    }
    Ok(())
}
