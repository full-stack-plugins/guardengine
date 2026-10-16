use crate::{InputEntry, git_client::GitClient};
use guard_protocol::{Digest, GitOid, GitPath};
use sha2::{Digest as Hasher, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Read,
};

pub(crate) const FILE_LIMIT: usize = 4096;
pub(crate) const BYTE_LIMIT: usize = 4 * 1024 * 1024;
const TOTAL_LIMIT: usize = 128 * 1024 * 1024;
pub(crate) fn digest(bytes: &[u8]) -> Result<Digest, String> {
    Digest::new(format!("sha256:{:x}", Sha256::digest(bytes)))
}
pub(crate) fn json_digest(value: &impl serde::Serialize) -> Result<Digest, String> {
    digest(&guard_protocol::canonicalize(
        &serde_json::to_vec(value).map_err(|e| e.to_string())?,
    )?)
}
fn mode(value: &str) -> Result<(), String> {
    match value {
        "100644" | "100755" => Ok(()),
        "120000" => Err("symlink Git entry unsupported".into()),
        "160000" => Err("submodule Git entry unsupported".into()),
        _ => Err(format!("unsupported Git entry mode: {value}")),
    }
}
fn parse(bytes: &[u8], index: bool) -> Result<BTreeMap<Vec<u8>, (String, GitOid)>, String> {
    let mut result = BTreeMap::new();
    for row in bytes.split(|v| *v == 0).filter(|v| !v.is_empty()) {
        let split = row
            .iter()
            .position(|v| *v == b'\t')
            .ok_or("invalid Git inventory record")?;
        let header =
            std::str::from_utf8(&row[..split]).map_err(|_| "invalid Git inventory header")?;
        let fields = header.split(' ').collect::<Vec<_>>();
        if fields.len() != 3 {
            return Err("invalid Git inventory fields".into());
        }
        mode(fields[0])?;
        if index && fields[2] != "0" {
            return Err("unmerged index unsupported".into());
        }
        if !index && fields[1] != "blob" {
            return Err("non-blob tree input unsupported".into());
        }
        let oid = GitOid::new(fields[if index { 1 } else { 2 }])?;
        let path = row[split + 1..].to_vec();
        GitPath::from_bytes(&path)?;
        if result.insert(path, (fields[0].into(), oid)).is_some() {
            return Err("duplicate Git inventory path".into());
        }
        if result.len() > FILE_LIMIT {
            return Err("Git inventory file limit exceeded".into());
        }
    }
    Ok(result)
}
pub(crate) fn index_entries(
    client: &GitClient,
) -> Result<BTreeMap<Vec<u8>, (String, GitOid)>, String> {
    parse(
        &client.run(&["ls-files", "--stage", "-z"], 1024 * 1024, false)?,
        true,
    )
}
pub(crate) fn tree(client: &GitClient, oid: &str) -> Result<Vec<InputEntry>, String> {
    // 先独立核验每层原始 tree 对象，再解释其路径和子对象，避免 ls-tree 读取被替换的对象。
    let mut stack = vec![(Vec::<u8>::new(), GitOid::new(oid)?, 0usize)];
    let mut entries = BTreeMap::new();
    let mut directories = 0;
    let mut tree_bytes = 0;
    while let Some((prefix, oid, depth)) = stack.pop() {
        directories += 1;
        if directories > FILE_LIMIT || depth > 64 {
            return Err("Git tree directory/depth limit exceeded".into());
        }
        let bytes = crate::git_object::read(client, "tree", &oid, BYTE_LIMIT)?;
        tree_bytes += bytes.len();
        if tree_bytes > TOTAL_LIMIT {
            return Err("Git tree byte limit exceeded".into());
        }
        let width = oid.as_str().len() / 2;
        let mut cursor = 0;
        let mut names = BTreeSet::new();
        let mut previous = None;
        while cursor < bytes.len() {
            let space = bytes[cursor..]
                .iter()
                .position(|v| *v == b' ')
                .ok_or("invalid raw tree mode")?
                + cursor;
            let nul = bytes[space + 1..]
                .iter()
                .position(|v| *v == 0)
                .ok_or("invalid raw tree name")?
                + space
                + 1;
            let end = nul
                .checked_add(width + 1)
                .filter(|v| *v <= bytes.len())
                .ok_or("truncated raw tree OID")?;
            let mode_text =
                std::str::from_utf8(&bytes[cursor..space]).map_err(|_| "invalid raw tree mode")?;
            let name = bytes[space + 1..nul].to_vec();
            if name.is_empty() || name.contains(&b'/') || !names.insert(name.clone()) {
                return Err("invalid or duplicate raw tree name".into());
            }
            let mut ordering = name.clone();
            ordering.push(if mode_text == "40000" { b'/' } else { 0 });
            if previous.as_ref().is_some_and(|v| v >= &ordering) {
                return Err("noncanonical raw tree ordering".into());
            }
            previous = Some(ordering);
            let mut path = prefix.clone();
            path.extend(name);
            GitPath::from_bytes(&path)?;
            let child = GitOid::new(
                bytes[nul + 1..end]
                    .iter()
                    .flat_map(|v| {
                        const HEX: &[u8; 16] = b"0123456789abcdef";
                        [
                            char::from(HEX[(v >> 4) as usize]),
                            char::from(HEX[(v & 15) as usize]),
                        ]
                    })
                    .collect::<String>(),
            )?;
            if mode_text == "40000" {
                path.push(b'/');
                stack.push((path, child, depth + 1));
            } else {
                mode(mode_text)?;
                if entries.insert(path, (mode_text.into(), child)).is_some()
                    || entries.len() > FILE_LIMIT
                {
                    return Err("duplicate Git path or file limit exceeded".into());
                }
            }
            cursor = end;
        }
    }
    objects(client, entries)
}
pub(crate) fn objects(
    client: &GitClient,
    entries: BTreeMap<Vec<u8>, (String, GitOid)>,
) -> Result<Vec<InputEntry>, String> {
    let mut total = 0;
    let mut result = Vec::new();
    for (path, (mode, oid)) in entries {
        let bytes = crate::git_object::read(client, "blob", &oid, BYTE_LIMIT)?;
        total += bytes.len();
        if total > TOTAL_LIMIT {
            return Err("Git input byte limit exceeded".into());
        }
        result.push(InputEntry {
            path: GitPath::from_bytes(&path)?,
            mode,
            object_id: Some(oid),
            content_digest: digest(&bytes)?,
        });
    }
    Ok(result)
}
pub(crate) fn worktree(client: &GitClient) -> Result<Vec<InputEntry>, String> {
    let tracked = index_entries(client)?;
    let flags = client.run(&["ls-files", "-v", "-z"], 1024 * 1024, false)?;
    if flags.split(|v| *v == 0).any(|v| {
        v.first()
            .is_some_and(|v| *v == b'S' || v.is_ascii_lowercase())
    }) {
        return Err("skip-worktree/assume-unchanged index flags unsupported".into());
    }
    let bytes = client.run(
        &["ls-files", "--cached", "--others", "-z"],
        1024 * 1024,
        false,
    )?;
    let mut entries = BTreeMap::new();
    let mut physical_paths = BTreeSet::new();
    let mut total = 0;
    for raw in bytes.split(|v| *v == 0).filter(|v| !v.is_empty()) {
        if entries.contains_key(raw) {
            return Err("duplicate worktree path".into());
        }
        let path = crate::paths::local_path(&client.root, raw)?;
        if !path.try_exists().map_err(|e| e.to_string())? {
            if tracked.contains_key(raw) {
                continue;
            }
            return Err("untracked input disappeared".into());
        }
        let canonical = crate::paths::safe_existing(&path)?;
        // Git 原始名称必须精确对应实际目录项；不能把 Windows 大小写或短文件名别名冒充新输入。
        // 不按内容或文件ID去重：两个真实目录项的硬链接仍是两个合法的输入路径。
        if canonical != path || !physical_paths.insert(canonical) {
            return Err("worktree path alias does not match an exact physical path".into());
        }
        let metadata = fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
        if !metadata.is_file() || metadata.len() > BYTE_LIMIT as u64 {
            return Err("unsupported or oversized worktree input".into());
        }
        let mut content = Vec::new();
        fs::File::open(&path)
            .map_err(|e| e.to_string())?
            .take(BYTE_LIMIT as u64 + 1)
            .read_to_end(&mut content)
            .map_err(|e| e.to_string())?;
        if content.len() > BYTE_LIMIT {
            return Err("worktree file byte limit exceeded".into());
        }
        total += content.len();
        if total > TOTAL_LIMIT {
            return Err("worktree input byte limit exceeded".into());
        }
        let mode = {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if metadata.permissions().mode() & 0o111 != 0 {
                    "100755"
                } else {
                    "100644"
                }
                .to_owned()
            }
            #[cfg(not(unix))]
            {
                tracked
                    .get(raw)
                    .map(|v| v.0.clone())
                    .unwrap_or_else(|| "100644".into())
            }
        };
        entries.insert(
            raw.to_vec(),
            InputEntry {
                path: GitPath::from_bytes(raw)?,
                mode,
                object_id: None,
                content_digest: digest(&content)?,
            },
        );
        if entries.len() > FILE_LIMIT {
            return Err("worktree file limit exceeded".into());
        }
    }
    Ok(entries.into_values().collect())
}
