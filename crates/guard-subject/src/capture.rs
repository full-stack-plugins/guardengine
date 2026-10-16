use crate::{CaptureBinding, CapturedSubject, Selection, git_client::GitClient, manifest};
use guard_protocol::{
    Extensions, GitOid, GitSubjectSnapshot, ObjectFormat, ProtocolDocument, SchemaVersion,
    SubjectKind, SubjectSnapshot,
};
use std::{fs, io::Read, path::Path};

/// 捕获真实候选并重复采集核对稳定性；参数为仓库根、显式绑定、精确选择。
/// 返回输入声明，不核验外部 repository/task 身份，不给予验收或执行权。
pub fn capture(
    project: &Path,
    binding: &CaptureBinding,
    selection: &Selection,
) -> Result<CapturedSubject, String> {
    let root = crate::paths::safe_existing(project)?;
    let scratch = tempfile::Builder::new()
        .prefix("guard-subject-")
        .tempdir()
        .map_err(|e| e.to_string())?;
    if scratch.path().starts_with(&root) {
        return Err("private capture scratch must be outside repository".into());
    }
    let client = GitClient::new(root, scratch.path())?;
    let first = once(&client, binding, selection)?;
    let second = once(&client, binding, selection)?;
    if first != second {
        return Err("Git inputs changed during capture".into());
    }
    Ok(first)
}
/// 对当前候选重新采集并逐字段比较；禁止 index/commit/merge 或其他任务互用。
pub fn verify_current(
    project: &Path,
    binding: &CaptureBinding,
    selection: &Selection,
    previous: &CapturedSubject,
) -> Result<(), String> {
    if &previous.binding != binding || &previous.selection != selection {
        return Err("subject binding or candidate kind changed".into());
    }
    if &capture(project, binding, selection)? != previous {
        return Err("subject stale or input changed".into());
    }
    Ok(())
}
fn commit_tree(client: &GitClient, oid: &GitOid) -> Result<String, String> {
    let bytes = crate::git_object::read(client, "commit", oid, manifest::BYTE_LIMIT)?;
    let line = bytes
        .split(|v| *v == b'\n')
        .next()
        .ok_or("missing commit tree")?;
    let tree = line
        .strip_prefix(b"tree ")
        .ok_or("invalid commit tree header")?;
    let tree = GitOid::new(std::str::from_utf8(tree).map_err(|_| "invalid commit tree OID")?)?;
    if tree.as_str().len() != oid.as_str().len() {
        return Err("commit/tree object format mismatch".into());
    }
    Ok(tree.as_str().into())
}
fn copy_index(client: &GitClient) -> Result<(), String> {
    let shared = client.text(&["rev-parse", "--shared-index-path"])?;
    if !shared.is_empty() {
        return Err("split index unsupported".into());
    }
    let path = client.text(&["rev-parse", "--git-path", "index"])?;
    let path = crate::paths::safe_existing(&client.root.join(path))?;
    if fs::metadata(&path).map_err(|e| e.to_string())?.len() > 8 * 1024 * 1024 {
        return Err("index byte limit exceeded".into());
    }
    let mut data = Vec::new();
    fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(8 * 1024 * 1024 + 1)
        .read_to_end(&mut data)
        .map_err(|e| e.to_string())?;
    if data.len() > 8 * 1024 * 1024 {
        return Err("index byte limit exceeded".into());
    }
    fs::write(&client.private_index, data).map_err(|e| e.to_string())
}
pub(crate) fn once(
    client: &GitClient,
    binding: &CaptureBinding,
    selection: &Selection,
) -> Result<CapturedSubject, String> {
    let object_format = match client
        .text(&["rev-parse", "--show-object-format"])?
        .as_str()
    {
        "sha1" => ObjectFormat::Sha1,
        "sha256" => ObjectFormat::Sha256,
        _ => return Err("unsupported Git object format".into()),
    };
    let (kind, base, head, tree, target_ref, expected, manifest) = match selection {
        Selection::Worktree { base } => {
            commit_tree(client, base)?;
            let head = GitOid::new(client.text(&["rev-parse", "--verify", "HEAD^{commit}"])?)?;
            (
                SubjectKind::Worktree,
                base.clone(),
                head,
                None,
                None,
                None,
                manifest::worktree(client)?,
            )
        }
        Selection::Index { base } => {
            commit_tree(client, base)?;
            let head = GitOid::new(client.text(&["rev-parse", "--verify", "HEAD^{commit}"])?)?;
            let entries = manifest::index_entries(client)?;
            copy_index(client)?;
            let tree = GitOid::new(
                String::from_utf8(client.run(&["write-tree"], 1024, true)?)
                    .map_err(|e| e.to_string())?
                    .trim(),
            )?;
            let manifest = manifest::objects(client, entries)?;
            // 私有 index 计算树后再核对其实际 blob，避免采集/复制间暂存区漂移。
            if manifest::tree(client, tree.as_str())? != manifest {
                return Err("index changed during tree calculation".into());
            }
            (
                SubjectKind::Index,
                base.clone(),
                head,
                Some(tree),
                None,
                None,
                manifest,
            )
        }
        Selection::Commit { commit, base } => {
            commit_tree(client, base)?;
            let tree = GitOid::new(commit_tree(client, commit)?)?;
            (
                SubjectKind::Commit,
                base.clone(),
                commit.clone(),
                Some(tree.clone()),
                None,
                None,
                manifest::tree(client, tree.as_str())?,
            )
        }
        Selection::MergeCandidate {
            head,
            target_ref,
            expected_target,
        } => {
            if !target_ref.as_str().starts_with("refs/heads/") {
                return Err("merge target must be a full local branch ref".into());
            }
            client.run(&["check-ref-format", target_ref.as_str()], 1024, false)?;
            let actual = client.text(&[
                "rev-parse",
                "--verify",
                &format!("{}^{{commit}}", target_ref.as_str()),
            ])?;
            if actual != expected_target.as_str() {
                return Err("merge target OID drift".into());
            }
            commit_tree(client, expected_target)?;
            commit_tree(client, head)?;
            // 不执行仓库配置中的自定义 merge driver；配置枚举本身不运行命令。
            let config = client.text(&["config", "--includes", "--list"])?;
            if config.lines().any(|line| {
                line.split('=').next().is_some_and(|name| {
                    name.starts_with("merge.") && name.ends_with(".driver")
                        || name.starts_with("filter.")
                            && [".clean", ".smudge", ".process"]
                                .iter()
                                .any(|suffix| name.ends_with(suffix))
                })
            }) {
                return Err("custom merge driver/filter unsupported".into());
            }
            if config.lines().any(|line| {
                line.split('=')
                    .next()
                    .is_some_and(|name| name.starts_with("fsck."))
            }) {
                return Err("repository fsck overrides unsupported".into());
            }
            // merge-base/递归合并可能读取祖先对象；先用 Git 真正的哈希/连通性检查拒绝损坏历史。
            client.run(
                &[
                    "fsck",
                    "--strict",
                    "--no-progress",
                    "--no-reflogs",
                    "--no-dangling",
                    head.as_str(),
                    expected_target.as_str(),
                ],
                1024 * 1024,
                true,
            )?;
            let bytes = client.run(
                &[
                    "merge-tree",
                    "--write-tree",
                    expected_target.as_str(),
                    head.as_str(),
                ],
                1024 * 1024,
                true,
            )?;
            let output = std::str::from_utf8(&bytes).map_err(|_| "invalid merge tree output")?;
            let tree = GitOid::new(output.lines().next().ok_or("missing merge tree")?)?;
            (
                SubjectKind::MergeCandidate,
                expected_target.clone(),
                head.clone(),
                Some(tree.clone()),
                Some(target_ref.clone()),
                Some(expected_target.clone()),
                manifest::tree(client, tree.as_str())?,
            )
        }
    };
    commit_tree(client, &head)?;
    let baseline = manifest::tree(client, &commit_tree(client, &base)?)?;
    let input_manifest_digest = manifest::json_digest(&manifest)?;
    let diff_digest = manifest::json_digest(&(base.clone(), baseline, &manifest))?;
    let snapshot: SubjectSnapshot = GitSubjectSnapshot {
        schema_version: SchemaVersion::new("1.0.0")?,
        kind,
        repository_id: binding.repository_id.clone(),
        worktree_id: if matches!(kind, SubjectKind::Index | SubjectKind::Worktree) {
            Some(binding.worktree_id.clone())
        } else {
            None
        },
        object_format,
        base: Some(base),
        head: Some(head),
        tree,
        target_ref,
        expected_target_oid: expected,
        input_manifest_digest,
        diff_digest: Some(diff_digest),
        extensions: Extensions::default(),
    }
    .into();
    guard_protocol::encode(&ProtocolDocument::SubjectSnapshot(snapshot.clone()))?;
    let identity_digest = manifest::json_digest(&(&snapshot, binding, selection, &manifest))?;
    Ok(CapturedSubject {
        snapshot,
        binding: binding.clone(),
        selection: selection.clone(),
        manifest,
        identity_digest,
    })
}
