use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};

pub(crate) fn safe_path(path: &Path) -> Result<PathBuf, String> {
    if !path.is_absolute() {
        return Err("absolute path required".into());
    }
    for ancestor in path.ancestors() {
        if let Ok(meta) = fs::symlink_metadata(ancestor) {
            if meta.file_type().is_symlink() {
                return Err("linked evidence path unsupported".into());
            }
            #[cfg(windows)]
            {
                use std::os::windows::fs::MetadataExt;
                if meta.file_attributes() & 0x400 != 0 {
                    return Err("reparse evidence path unsupported".into());
                }
            }
        }
    }
    path.canonicalize().map_err(|e| e.to_string())
}
pub(crate) fn read(path: &Path, limit: u64) -> Result<Vec<u8>, String> {
    let resolved = safe_path(path)?;
    let before = fs::metadata(&resolved).map_err(|e| e.to_string())?;
    if !before.is_file() || before.len() > limit {
        return Err("evidence input is not a bounded regular file".into());
    }
    let file = fs::File::open(&resolved).map_err(|e| e.to_string())?;
    let opened = file.metadata().map_err(|e| e.to_string())?;
    if !opened.is_file() || opened.len() > limit {
        return Err("evidence input changed before open".into());
    }
    let mut bytes = Vec::new();
    file.take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > limit
        || safe_path(path)? != resolved
        || fs::metadata(&resolved).map_err(|e| e.to_string())?.len() != bytes.len() as u64
    {
        return Err("evidence input changed or exceeded budget".into());
    }
    Ok(bytes)
}
/// 读取最大 4MiB 的普通 JSON 证据；路径必须绝对且不含链接。
pub fn read_json(path: &Path) -> Result<Value, String> {
    serde_json::from_slice(&read(path, 4 * 1024 * 1024)?).map_err(|e| e.to_string())
}
/// 对显式普通文件计算 SHA256；最大 128MiB，用于执行程序、日志和产物字节。
pub fn hash_file(path: &Path) -> Result<String, String> {
    Ok(format!(
        "sha256:{:x}",
        Sha256::digest(read(path, 128 * 1024 * 1024)?)
    ))
}
