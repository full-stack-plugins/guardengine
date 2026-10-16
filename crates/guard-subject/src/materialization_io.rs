use crate::{InputEntry, manifest, paths};
use std::{
    collections::BTreeSet,
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

pub(crate) fn path(root: &Path, raw: &[u8]) -> Result<PathBuf, String> {
    // 所有层级拒绝Git控制目录及常见NTFS别名，不能复制gitdir指针读回原始仓库。
    for part in raw.split(|b| *b == b'/') {
        let part = part.trim_ascii_end();
        let mut end = part.len();
        while end > 0 && (part[end - 1] == b'.' || part[end - 1] == b' ') {
            end -= 1;
        }
        let folded = part[..end].to_ascii_lowercase();
        if folded == b".git"
            || folded == b"git~1"
            || folded.starts_with(b".git:")
            || folded.starts_with(b"git~1:")
        {
            return Err("Git control path or alias cannot be materialized".into());
        }
    }
    paths::local_path(root, raw)
}

pub(crate) fn read(root: &Path, entry: &InputEntry) -> Result<Vec<u8>, String> {
    let path = path(root, &entry.path.to_bytes())?;
    let canonical = paths::safe_existing(&path)?;
    if canonical != path {
        return Err("materialized input path alias".into());
    }
    let meta = fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
    if !meta.is_file() || meta.len() > manifest::BYTE_LIMIT as u64 {
        return Err("invalid or oversized materialized input".into());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = if meta.permissions().mode() & 0o111 != 0 {
            "100755"
        } else {
            "100644"
        };
        if mode != entry.mode {
            return Err("materialized input mode changed".into());
        }
    }
    let mut bytes = Vec::new();
    fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(manifest::BYTE_LIMIT as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > manifest::BYTE_LIMIT || manifest::digest(&bytes)? != entry.content_digest {
        return Err("materialized input bytes changed".into());
    }
    Ok(bytes)
}

pub(crate) fn write(root: &Path, entry: &InputEntry, bytes: &[u8]) -> Result<(), String> {
    let destination = path(root, &entry.path.to_bytes())?;
    let relative = destination.strip_prefix(root).map_err(|e| e.to_string())?;
    let mut parent = root.to_path_buf();
    let components: Vec<_> = relative.components().collect();
    for part in &components[..components.len() - 1] {
        parent.push(part.as_os_str());
        match fs::create_dir(&parent) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e.to_string()),
        }
        if paths::safe_existing(&parent)? != parent || !parent.is_dir() {
            return Err("materialization directory alias".into());
        }
    }
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&destination)
        .map_err(|e| format!("materialization file path collision or write failure: {e}"))?;
    file.write_all(bytes).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        file.set_permissions(fs::Permissions::from_mode(if entry.mode == "100755" {
            0o755
        } else {
            0o644
        }))
        .map_err(|e| e.to_string())?;
    }
    if paths::safe_existing(&destination)? != destination {
        return Err("materialization file alias".into());
    }
    Ok(())
}

pub(crate) fn verify(root: &Path, entries: &[InputEntry]) -> Result<(), String> {
    let root = paths::safe_existing(root)?;
    let mut canonical = BTreeSet::new();
    for entry in entries {
        read(&root, entry)?;
        if !canonical.insert(paths::safe_existing(&path(&root, &entry.path.to_bytes())?)?) {
            return Err("materialized input path collision".into());
        }
    }
    Ok(())
}
