use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
};
/// 验证本机路径祖先无符号链接/Windows reparse，返回规范绝对路径。
pub(crate) fn safe_existing(path: &Path) -> Result<PathBuf, String> {
    if !path.is_absolute() {
        return Err("local path must be absolute".into());
    }
    for ancestor in path.ancestors() {
        let metadata = fs::symlink_metadata(ancestor).map_err(|e| e.to_string())?;
        if metadata.file_type().is_symlink() {
            return Err("symlink input is unsupported".into());
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            if metadata.file_attributes() & 0x400 != 0 {
                return Err("reparse input is unsupported".into());
            }
        }
    }
    path.canonicalize().map_err(|e| e.to_string())
}
/// 将原始 Git 路径用于本机读取；Windows 不支持不可表示名称，绝不损失解码。
pub(crate) fn local_path(root: &Path, raw: &[u8]) -> Result<PathBuf, String> {
    guard_protocol::GitPath::from_bytes(raw)?;
    #[cfg(unix)]
    let name = {
        use std::os::unix::ffi::OsStringExt;
        OsString::from_vec(raw.to_vec())
    };
    #[cfg(not(unix))]
    let name = {
        let value = std::str::from_utf8(raw)
            .map_err(|_| "non-UTF8 worktree path unsupported on this platform")?;
        if value.contains(['\\', ':']) {
            return Err("Windows path alias is unsupported".into());
        }
        OsString::from(value)
    };
    let path = root.join(name);
    if !path.starts_with(root) {
        return Err("input escapes repository".into());
    }
    Ok(path)
}
