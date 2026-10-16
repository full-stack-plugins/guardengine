use crate::{
    digest_json,
    file_input::{read, safe_path},
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

const EXCLUDED: [&str; 4] = [".git", "target", "node_modules", ".guard-build"];
fn inventory(root: &Path) -> Result<BTreeMap<String, PathBuf>, String> {
    let mut pending = vec![(root.to_path_buf(), 0)];
    let mut directories = 0;
    let mut files = BTreeMap::new();
    while let Some((directory, depth)) = pending.pop() {
        directories += 1;
        if directories > 16384 || depth > 64 {
            return Err("source directory budget exceeded".into());
        }
        safe_path(&directory)?;
        for entry in fs::read_dir(&directory).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| "non-UTF8 source path")?;
            if EXCLUDED
                .iter()
                .any(|excluded| name.eq_ignore_ascii_case(excluded))
            {
                continue;
            }
            let path = entry.path();
            let canonical = safe_path(&path)?;
            if !canonical.starts_with(root) {
                return Err("source path escapes project".into());
            }
            let meta = entry.metadata().map_err(|e| e.to_string())?;
            if meta.is_dir() {
                pending.push((path, depth + 1));
            } else if meta.is_file() {
                if files.len() >= 8192 {
                    return Err("source file budget exceeded".into());
                }
                files.insert(
                    path.strip_prefix(root)
                        .map_err(|e| e.to_string())?
                        .to_string_lossy()
                        .replace('\\', "/"),
                    path,
                );
            } else {
                return Err("non-regular source path".into());
            }
        }
    }
    Ok(files)
}
/// 创建完整声明范围的当前项目快照；排除构建目录但不排除源码或计划。
pub fn snapshot_project(project: &Path) -> Result<Value, String> {
    let root = safe_path(project)?;
    if !root.is_dir() {
        return Err("project directory required".into());
    }
    let inventory_before = inventory(&root)?;
    let mut sources = Vec::new();
    let mut observed = BTreeMap::new();
    let mut total = 0;
    for (name, path) in &inventory_before {
        let bytes = read(path, 4 * 1024 * 1024)?;
        total += bytes.len();
        if total > 128 * 1024 * 1024 {
            return Err("source snapshot exceeds 128 MiB".into());
        }
        let hash = format!("sha256:{:x}", Sha256::digest(&bytes));
        sources.push(json!({"path":name,"sha256":hash}));
        observed.insert(name, hash);
    }
    if inventory_before != inventory(&root)? {
        return Err("source inventory changed during snapshot".into());
    }
    for (name, path) in &inventory_before {
        let hash = format!("sha256:{:x}", Sha256::digest(read(path, 4 * 1024 * 1024)?));
        if observed[name] != hash {
            return Err("source changed during snapshot".into());
        }
    }
    let scope = json!({"excluded_directory_names":EXCLUDED,"file_limit":8192,"file_byte_limit":4*1024*1024,"total_byte_limit":128*1024*1024,"coverage":"project files, not Git identity, runtime environment or external toolchain inputs"});
    let digest = digest_json(
        &json!({"schema_version":"guard.source-snapshot/v1","files":sources,"scope":scope}),
    )?;
    Ok(
        json!({"schema_version":"guard.source-snapshot/v1","digest":digest,"files":sources,"scope":scope}),
    )
}
