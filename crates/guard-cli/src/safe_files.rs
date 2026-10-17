use serde::de::DeserializeOwned;
use std::{
    fs,
    io::Read,
    path::{Component, Path, PathBuf},
};
/// 检查绝对路径每段普通性，返回规范路径或拒绝原因。
pub(crate) fn plain(path: &Path) -> Result<PathBuf, String> {
    #[cfg(windows)]
    let path_buf = portable_windows_path(path);
    #[cfg(windows)]
    let path = path_buf.as_path();
    if !path.is_absolute() {
        return Err("absolute path required".into());
    }
    let mut prefix = PathBuf::new();
    for part in path.components() {
        if matches!(part, Component::ParentDir | Component::CurDir) {
            return Err("path traversal refused".into());
        }
        prefix.push(part);
        let meta = fs::symlink_metadata(&prefix).map_err(|e| format!("path unavailable: {e}"))?;
        if meta.file_type().is_symlink() {
            return Err("linked path refused".into());
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            if meta.file_attributes() & 0x400 != 0 {
                return Err("reparse path refused".into());
            }
        }
    }
    path.canonicalize().map_err(|e| e.to_string())
}
#[cfg(windows)]
fn portable_windows_path(path: &Path) -> PathBuf {
    let raw = path.as_os_str().to_string_lossy();
    if let Some(tail) = raw.strip_prefix("\\\\?\\UNC\\") {
        PathBuf::from(format!("\\\\{tail}"))
    } else if let Some(tail) = raw.strip_prefix("\\\\?\\") {
        PathBuf::from(tail)
    } else {
        path.to_path_buf()
    }
}
/// 按路径与字节上限读取普通文件，返回原始字节。
pub(crate) fn read(path: &Path, limit: u64) -> Result<Vec<u8>, String> {
    plain(path)?;
    if !fs::symlink_metadata(path)
        .map_err(|e| e.to_string())?
        .is_file()
    {
        return Err("regular file required before opening".into());
    }
    #[cfg(unix)]
    let file = {
        use std::os::unix::fs::OpenOptionsExt;
        // 即使检查后被换成FIFO，也不能在open等待写端；仍必须再核对句柄类型。
        fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(path)
            .map_err(|e| e.to_string())?
    };
    #[cfg(not(unix))]
    let file = fs::File::open(path).map_err(|e| e.to_string())?;
    if !file.metadata().map_err(|e| e.to_string())?.is_file() {
        return Err("regular file required after opening".into());
    }
    let mut bytes = Vec::new();
    file.take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > limit {
        return Err("input exceeds byte budget".into());
    }
    Ok(bytes)
}
/// 把宿主根内相对名称解析为受限普通路径，返回绝对路径。
pub(crate) fn relative(root: &Path, name: &str) -> Result<PathBuf, String> {
    if name.is_empty()
        || name.contains('\\')
        || name
            .split('/')
            .any(|x| x.is_empty() || x == "." || x == "..")
        || Path::new(name)
            .components()
            .any(|x| !matches!(x, Component::Normal(_)))
    {
        return Err("host-relative ordinary path required".into());
    }
    let p = plain(&root.join(name))?;
    if !p.starts_with(root) {
        return Err("host path escaped fixed root".into());
    }
    Ok(p)
}
/// 校验重复键并解码输入字节，返回指定类型。
pub(crate) fn decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, String> {
    serde_json::from_slice(&guard_protocol::canonicalize(bytes)?).map_err(|e| e.to_string())
}
/// 按字段名解码闭合配置记录中的指定字段，返回类型值。
pub(crate) fn get<T: DeserializeOwned>(value: &serde_json::Value, key: &str) -> Result<T, String> {
    serde_json::from_value(
        value
            .get(key)
            .ok_or_else(|| format!("missing field: {key}"))?
            .clone(),
    )
    .map_err(|e| format!("{key}: {e}"))
}

#[cfg(test)]
mod tests {
    use super::{decode, read, relative};
    #[test]
    fn malformed_duplicate_unknown_and_link_paths_are_not_host_authority() {
        assert!(decode::<serde_json::Value>(br#"{"a":1,"a":2}"#).is_err());
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().canonicalize().unwrap();
        std::fs::write(root.join("record.json"), b"{}").unwrap();
        assert!(relative(&root, "../record.json").is_err());
        assert!(relative(&root, "sub//record.json").is_err());
        assert!(read(&root.join("record.json"), 1).is_err());
        let normal = relative(&root, "record.json").unwrap();
        assert_eq!(read(&normal, 2).unwrap(), b"{}");
    }
    #[cfg(unix)]
    #[test]
    fn named_pipe_is_rejected_before_open_can_wait_for_a_writer() {
        use std::{ffi::CString, os::unix::ffi::OsStrExt, sync::mpsc, time::Duration};
        let dir = tempfile::tempdir().unwrap();
        let fifo = dir.path().join("input.fifo");
        let name = CString::new(fifo.as_os_str().as_bytes()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
        let (send, receive) = mpsc::channel();
        let copy = fifo.clone();
        let worker = std::thread::spawn(move || {
            send.send(read(&copy, 1024)).unwrap();
        });
        let early = receive.recv_timeout(Duration::from_millis(500));
        // 若旧实现先阻塞open，写端只负责解除阻塞，随后仍断言该实现未及时拒绝。
        if early.is_err() {
            let _ = std::fs::OpenOptions::new().write(true).open(&fifo);
        }
        worker.join().unwrap();
        assert!(early.expect("FIFO must reject before opening").is_err());
    }
}
