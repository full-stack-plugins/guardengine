use crate::mutation::Mutation;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
pub(crate) const MAX_BYTES: u64 = 2 * 1024 * 1024;

pub(crate) fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub(crate) fn safe_path(path: &Path) -> Result<(), String> {
    if !path.is_absolute()
        || path
            .components()
            .any(|p| matches!(p, std::path::Component::ParentDir))
    {
        return Err("path must be absolute without parent traversal".into());
    }
    for ancestor in path.ancestors() {
        match fs::symlink_metadata(ancestor) {
            Ok(meta) => {
                #[cfg(windows)]
                {
                    use std::os::windows::fs::MetadataExt;
                    if meta.file_attributes() & 0x400 != 0 {
                        return Err(format!("reparse point rejected: {}", ancestor.display()));
                    }
                }
                if meta.file_type().is_symlink() {
                    return Err(format!("symlink rejected: {}", ancestor.display()));
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.to_string()),
        }
    }
    Ok(())
}

pub(crate) fn read(path: &Path, limit: u64) -> Result<Option<Vec<u8>>, String> {
    safe_path(path)?;
    let meta = match fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.to_string()),
    };
    if !meta.is_file() || meta.len() > limit {
        return Err(format!("not a bounded regular file: {}", path.display()));
    }
    let mut bytes = vec![];
    fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > limit {
        return Err("file grew beyond limit".into());
    }
    Ok(Some(bytes))
}

fn replace(path: &Path, bytes: Option<&[u8]>, executable: bool) -> Result<(), String> {
    safe_path(path)?;
    if let Some(bytes) = bytes {
        let parent = path.parent().ok_or("missing parent")?;
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        let temp = parent.join(format!(
            ".guard-tmp-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let result = (|| {
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temp)
                .map_err(|e| e.to_string())?;
            file.write_all(bytes).map_err(|e| e.to_string())?;
            if let Ok(meta) = fs::metadata(path) {
                file.set_permissions(meta.permissions())
                    .map_err(|e| e.to_string())?;
            }
            #[cfg(unix)]
            if executable {
                use std::os::unix::fs::PermissionsExt;
                file.set_permissions(fs::Permissions::from_mode(0o755))
                    .map_err(|e| e.to_string())?;
            }
            #[cfg(not(unix))]
            let _ = executable;
            file.sync_all().map_err(|e| e.to_string())?;
            drop(file);
            fs::rename(&temp, path).map_err(|e| e.to_string())
        })();
        if temp.exists() {
            let _ = fs::remove_file(&temp);
        }
        result
    } else {
        fs::remove_file(path).map_err(|e| e.to_string())
    }
}

pub(crate) fn apply(changes: &[Mutation], state: &Path, dry_run: bool) -> Result<(), String> {
    if dry_run {
        return Ok(());
    }
    safe_path(state)?;
    fs::create_dir_all(state).map_err(|e| e.to_string())?;
    let lock = state.join(".lock");
    let handle = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&lock)
        .map_err(|_| "installation locked; inspect unfinished operation before retry".to_owned())?;
    let result = (|| {
        let mut written: Vec<&Mutation> = vec![];
        for change in changes {
            if change.before == change.after {
                continue;
            }
            let operation = (|| {
                if read(&change.path, 512 * 1024 * 1024)? != change.before {
                    return Err(format!(
                        "file changed after planning: {}",
                        change.path.display()
                    ));
                }
                replace(&change.path, change.after.as_deref(), change.executable)
            })();
            if let Err(error) = operation {
                let mut failures = vec![];
                for prior in written.into_iter().rev() {
                    if read(&prior.path, 512 * 1024 * 1024).ok().as_ref() != Some(&prior.after)
                        || replace(&prior.path, prior.before.as_deref(), prior.executable).is_err()
                    {
                        failures.push(prior.path.display().to_string());
                    }
                }
                return Err(format!("{error}; rollback conflicts: {failures:?}"));
            }
            written.push(change);
        }
        Ok(())
    })();
    drop(handle);
    let cleanup = fs::remove_file(lock).map_err(|e| e.to_string());
    result.and(cleanup)
}

#[cfg(test)]
mod tests {
    use super::apply;
    use crate::mutation::Mutation;
    use std::fs;

    #[test]
    fn later_concurrent_change_rolls_back_earlier_write() {
        let root = std::env::temp_dir().join(format!("guard-rollback-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let first = root.join("first");
        let second = root.join("second");
        fs::write(&first, b"original").unwrap();
        fs::write(&second, b"concurrent edit").unwrap();
        let changes = [
            Mutation {
                path: first.clone(),
                before: Some(b"original".to_vec()),
                after: Some(b"generated".to_vec()),
                executable: false,
            },
            Mutation {
                path: second.clone(),
                before: Some(b"outdated observation".to_vec()),
                after: Some(b"generated".to_vec()),
                executable: false,
            },
        ];
        let result = apply(&changes, &root.join("state"), false);
        assert!(result.unwrap_err().contains("file changed after planning"));
        assert_eq!(fs::read(&first).unwrap(), b"original");
        assert_eq!(fs::read(&second).unwrap(), b"concurrent edit");
        assert!(!root.join("state/.lock").exists());
        fs::remove_dir_all(&root).unwrap();
    }
}
