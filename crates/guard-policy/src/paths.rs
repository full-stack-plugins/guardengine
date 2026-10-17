use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};
pub(crate) fn relative(path: &str) -> Result<(), String> {
    if path.is_empty()
        || path.len() > 1024
        || path.contains('\\')
        || path.split('/').any(|p| {
            p.is_empty()
                || p == "."
                || p == ".."
                || p.ends_with('.')
                || !p
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.'))
                || {
                    let head = p.split('.').next().unwrap_or("").to_ascii_uppercase();
                    [
                        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6",
                        "COM7", "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6",
                        "LPT7", "LPT8", "LPT9",
                    ]
                    .contains(&head.as_str())
                }
        })
    {
        return Err("portable literal relative policy file required".into());
    }
    Ok(())
}
pub(crate) fn safe(path: &Path) -> Result<PathBuf, String> {
    if !path.is_absolute() {
        return Err("absolute policy path required".into());
    }
    for parent in path.ancestors() {
        let m = fs::symlink_metadata(parent).map_err(|e| e.to_string())?;
        if m.file_type().is_symlink() {
            return Err("linked policy path rejected".into());
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            if m.file_attributes() & 0x400 != 0 {
                return Err("reparse policy path rejected".into());
            }
        }
    }
    path.canonicalize().map_err(|e| e.to_string())
}
pub(crate) fn read(path: &Path, max: u64) -> Result<Vec<u8>, String> {
    let path = safe(path)?;
    let meta = fs::metadata(&path).map_err(|e| e.to_string())?;
    if !meta.is_file() || meta.len() > max {
        return Err("bounded regular policy file required".into());
    }
    let mut file = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    file.by_ref()
        .take(max + 1)
        .read_to_end(&mut out)
        .map_err(|e| e.to_string())?;
    if out.len() as u64 > max || out.len() as u64 != meta.len() {
        return Err("policy file changed beyond size budget".into());
    }
    Ok(out)
}
pub(crate) fn native(path: &Path) -> Result<PathBuf, String> {
    let text = path.to_str().ok_or("Unicode control path required")?;
    #[cfg(windows)]
    {
        if let Some(tail) = text.strip_prefix(r"\\?\UNC\") {
            return Ok(PathBuf::from(format!(r"\\{}", tail)));
        }
        if let Some(tail) = text.strip_prefix(r"\\?\") {
            if tail.as_bytes().get(1) == Some(&b':') {
                return Ok(PathBuf::from(tail));
            }
            return Err("unsupported policy control namespace".into());
        }
    }
    Ok(PathBuf::from(text))
}
