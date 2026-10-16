//! 原生规格的有界只读适配；不初始化体系，不批准需求，不复制规格正文。
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Read,
    path::{Component, Path, PathBuf},
};

/// 读取项目内普通文件；参数为项目根和相对路径，返回有界字节，拒绝链接与越界。
pub fn read_project_file(root: &Path, relative: &str) -> Result<Vec<u8>, String> {
    let path = contained(root, relative)?;
    if !path.is_file() {
        return Err(format!("not a regular file: {relative}"));
    }
    let mut bytes = Vec::new();
    fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 1024 * 1024 {
        return Err("specification file exceeds 1 MiB".into());
    }
    Ok(bytes)
}
fn contained(root: &Path, relative: &str) -> Result<PathBuf, String> {
    let rel = Path::new(relative);
    if relative.is_empty()
        || relative.contains('\\')
        || rel.components().any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err("spec path must be a nonempty project-relative path without traversal".into());
    }
    let mut path = root.to_path_buf();
    for part in rel.components() {
        path.push(part);
        match fs::symlink_metadata(&path) {
            Ok(meta) => {
                let linked = meta.file_type().is_symlink();
                #[cfg(windows)]
                let linked = {
                    use std::os::windows::fs::MetadataExt;
                    linked || meta.file_attributes() & 0x400 != 0
                };
                if linked {
                    return Err("linked specification path is unsupported".into());
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.to_string()),
        }
    }
    Ok(path)
}
fn children(root: &Path, relative: &str, directories: bool) -> Result<Vec<String>, String> {
    let path = contained(root, relative)?;
    if !path.exists() {
        return Ok(vec![]);
    }
    let mut result = vec![];
    for (index, entry) in fs::read_dir(path).map_err(|e| e.to_string())?.enumerate() {
        if index >= 1024 {
            return Err("spec directory entry budget exceeded".into());
        }
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| "non-UTF8 spec path")?;
        let relative = format!("{relative}/{name}");
        let checked = contained(root, &relative)?;
        if (directories && checked.is_dir())
            || (!directories && checked.is_file() && checked.extension().is_some_and(|s| s == "md"))
        {
            result.push(relative);
        }
    }
    result.sort();
    Ok(result)
}
fn markdown(
    root: &Path,
    relative: &str,
    depth: usize,
    files: &mut Vec<String>,
    directories_left: &mut usize,
) -> Result<(), String> {
    if depth > 8 || files.len() > 128 || *directories_left == 0 {
        return Err("spec scan budget exceeded".into());
    }
    *directories_left -= 1;
    files.extend(children(root, relative, false)?);
    for child in children(root, relative, true)? {
        markdown(root, &child, depth + 1, files, directories_left)?;
    }
    Ok(())
}

/// 发现或选择已有原生规格。system/target 仅作选择，返回状态、路径和摘要，不写文件。
pub fn specification_context(
    project: &Path,
    system: Option<&str>,
    target: Option<&str>,
) -> Result<Value, String> {
    let root = project.canonicalize().map_err(|e| e.to_string())?;
    let mut systems = vec![];
    for (name, marker) in [
        ("openspec", "openspec"),
        ("speckit", ".specify"),
        ("superpowers", "docs/superpowers/specs"),
    ] {
        if contained(&root, marker)?.is_dir() {
            systems.push(name);
        }
    }
    let selected = match system {
        Some(name) if systems.contains(&name) => name,
        Some(_) => return Err("requested specification system is absent or unsupported".into()),
        None if systems.is_empty() => {
            return Ok(json!({"status":"unconfigured","systems":[],"approved":false}));
        }
        None if systems.len() != 1 => {
            return Ok(
                json!({"status":"ambiguous","systems":systems,"reason":"select --spec-system and --spec-target","approved":false}),
            );
        }
        None => systems[0],
    };
    let candidates = match selected {
        "openspec" => children(&root, "openspec/changes", true)?
            .into_iter()
            .filter(|s| !s.ends_with("/archive"))
            .collect::<Vec<_>>(),
        "speckit" => children(&root, "specs", true)?,
        _ => children(&root, "docs/superpowers/specs", false)?,
    };
    let chosen = match target {
        Some(value) if candidates.iter().any(|v| v == value) => value.to_owned(),
        Some(_) => {
            return Err(
                "spec target must be an existing native candidate (archives are excluded)".into(),
            );
        }
        None if candidates.len() == 1 => candidates[0].clone(),
        None => {
            return Ok(
                json!({"status":if candidates.is_empty(){"incomplete"}else{"ambiguous"},"system":selected,"candidates":candidates,"reason":"select an existing --spec-target","approved":false}),
            );
        }
    };
    let mut files = vec![];
    let required = match selected {
        "openspec" => {
            markdown(&root, &format!("{chosen}/specs"), 0, &mut files, &mut 128)?;
            vec![
                format!("{chosen}/proposal.md"),
                format!("{chosen}/design.md"),
                format!("{chosen}/tasks.md"),
            ]
        }
        "speckit" => vec![
            format!("{chosen}/spec.md"),
            format!("{chosen}/plan.md"),
            format!("{chosen}/tasks.md"),
            ".specify/memory/constitution.md".into(),
        ],
        _ => {
            let plan = chosen.replace("/specs/", "/plans/");
            if contained(&root, &plan)?.is_file() {
                files.push(plan);
            }
            vec![chosen.clone()]
        }
    };
    let mut missing = vec![];
    if selected == "openspec" && !files.iter().any(|p| p.ends_with("/spec.md")) {
        missing.push(format!("{chosen}/specs/<capability>/spec.md"));
    }
    for path in &required {
        if contained(&root, path)?.is_file() {
            files.push(path.clone());
        } else {
            missing.push(path.clone());
        }
    }
    files.sort();
    files.dedup();
    if files.len() > 128 {
        return Err("spec artifact budget exceeded".into());
    }
    let mut total = 0;
    let mut artifacts = vec![];
    for path in files {
        let bytes = read_project_file(&root, &path)?;
        total += bytes.len();
        if total > 8 * 1024 * 1024 {
            return Err("specification total exceeds 8 MiB".into());
        }
        let text = std::str::from_utf8(&bytes).map_err(|_| "specification must be UTF-8")?;
        if text.trim().is_empty() {
            missing.push(path.clone());
        }
        artifacts.push(json!({"path":path,"sha256":format!("{:x}", Sha256::digest(&bytes))}));
    }
    let digest = format!(
        "sha256:{:x}",
        Sha256::digest(
            serde_json::to_vec(&json!({"system":selected,"target":chosen,"artifacts":artifacts}))
                .unwrap()
        )
    );
    Ok(
        json!({"status":if missing.is_empty(){"ready"}else{"incomplete"},"system":selected,"target":chosen,"systems":systems,"artifacts":artifacts,"missing":missing,"digest":digest,"approved":false,
        "guidance":"Edit the selected native artifacts; do not create duplicate specifications. ready means files are present, not semantic validation or user approval."}),
    )
}
