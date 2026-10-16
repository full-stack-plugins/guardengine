use guard_protocol::{Digest, GitOid, Identifier, ObjectFormat, SubjectKind};
use guard_subject::{CaptureBinding, Selection, capture, verify_current};
use std::{fs, path::Path, process::Command};

fn git(root: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env(
            "GIT_CONFIG_GLOBAL",
            if cfg!(windows) { "NUL" } else { "/dev/null" },
        )
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{:?}: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().into()
}
fn binding() -> CaptureBinding {
    let digest = Digest::new(format!("sha256:{}", "a".repeat(64))).unwrap();
    CaptureBinding {
        repository_id: Identifier::new("external-registry:example").unwrap(),
        task_id: Identifier::new("REQ-123").unwrap(),
        worktree_id: Identifier::new("declared-worktree-1").unwrap(),
        contract_digest: digest.clone(),
        policy_digest: digest.clone(),
        verifier_digest: digest.clone(),
        environment_digest: digest,
    }
}
fn fixture(format: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    git(dir.path(), &["init", &format!("--object-format={format}")]);
    git(dir.path(), &["config", "user.name", "Subject Test"]);
    git(
        dir.path(),
        &["config", "user.email", "subject@example.invalid"],
    );
    git(dir.path(), &["config", "core.autocrlf", "false"]);
    fs::write(dir.path().join("before.txt"), "original\n").unwrap();
    git(dir.path(), &["add", "before.txt"]);
    git(dir.path(), &["commit", "-m", "initial"]);
    dir
}
fn oid(root: &Path, name: &str) -> GitOid {
    GitOid::new(git(root, &["rev-parse", name])).unwrap()
}

fn object_files(directory: &Path) -> Vec<std::path::PathBuf> {
    let mut result = Vec::new();
    for entry in fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            result.extend(object_files(&path));
        } else {
            result.push(path);
        }
    }
    result.sort();
    result
}

#[test]
fn real_sha1_sha256_index_commit_worktree_are_distinct_and_current() {
    for format in ["sha1", "sha256"] {
        let dir = fixture(format);
        let root = dir.path();
        let head = oid(root, "HEAD");
        let committed = Selection::Commit {
            commit: head.clone(),
            base: head.clone(),
        };
        let initial = capture(root, &binding(), &committed).unwrap();
        assert_eq!(initial.snapshot.as_git().unwrap().kind, SubjectKind::Commit);
        assert_eq!(
            initial.snapshot.as_git().unwrap().object_format,
            if format == "sha1" {
                ObjectFormat::Sha1
            } else {
                ObjectFormat::Sha256
            }
        );
        fs::rename(root.join("before.txt"), root.join("after.txt")).unwrap();
        git(root, &["add", "-A"]);
        git(root, &["update-index", "--chmod=+x", "after.txt"]);
        let staged = Selection::Index { base: head.clone() };
        let index = capture(root, &binding(), &staged).unwrap();
        assert_ne!(index.identity_digest, initial.identity_digest);
        assert_eq!(index.manifest[0].path.to_bytes(), b"after.txt");
        assert_eq!(index.manifest[0].mode, "100755");
        fs::write(root.join("after.txt"), "not staged\n").unwrap();
        let worktree = capture(root, &binding(), &Selection::Worktree { base: head }).unwrap();
        assert_ne!(worktree.identity_digest, index.identity_digest);
        verify_current(root, &binding(), &staged, &index).unwrap();
        assert!(verify_current(root, &binding(), &committed, &index).is_err());
        assert_eq!(capture(root, &binding(), &committed).unwrap(), initial);
        let mut changed = binding();
        changed.task_id = Identifier::new("other-task").unwrap();
        assert!(verify_current(root, &changed, &staged, &index).is_err());
    }
}

#[test]
fn merge_candidate_is_actual_union_and_target_drift_invalidates() {
    let dir = fixture("sha1");
    let root = dir.path();
    let base = oid(root, "HEAD");
    git(root, &["branch", "target"]);
    fs::write(root.join("feature.txt"), "feature\n").unwrap();
    git(root, &["add", "."]);
    git(root, &["commit", "-m", "feature"]);
    let head = oid(root, "HEAD");
    git(root, &["checkout", "target"]);
    fs::write(root.join("target.txt"), "target\n").unwrap();
    git(root, &["add", "."]);
    git(root, &["commit", "-m", "target"]);
    let target = oid(root, "HEAD");
    let selection = Selection::MergeCandidate {
        head,
        target_ref: Identifier::new("refs/heads/target").unwrap(),
        expected_target: target.clone(),
    };
    let index_before = fs::read(root.join(".git/index")).unwrap();
    let objects_before = object_files(&root.join(".git/objects"));
    let candidate = capture(root, &binding(), &selection).unwrap();
    assert_eq!(candidate.manifest.len(), 3);
    assert_eq!(oid(root, "HEAD"), target);
    assert_eq!(fs::read(root.join(".git/index")).unwrap(), index_before);
    assert_eq!(object_files(&root.join(".git/objects")), objects_before);
    assert!(!root.join("feature.txt").exists());
    git(root, &["update-ref", "refs/heads/target", base.as_str()]);
    assert!(verify_current(root, &binding(), &selection, &candidate).is_err());
}

#[test]
fn submodule_and_custom_merge_driver_are_explicitly_rejected() {
    let dir = fixture("sha1");
    let root = dir.path();
    let head = oid(root, "HEAD");
    git(
        root,
        &[
            "update-index",
            "--add",
            "--cacheinfo",
            &format!("160000,{},module", head.as_str()),
        ],
    );
    assert!(
        capture(root, &binding(), &Selection::Index { base: head.clone() })
            .unwrap_err()
            .contains("submodule")
    );
    git(root, &["update-index", "--force-remove", "module"]);
    git(root, &["branch", "target"]);
    git(
        root,
        &["config", "merge.danger.driver", "echo should-never-run"],
    );
    let selection = Selection::MergeCandidate {
        head: head.clone(),
        target_ref: Identifier::new("refs/heads/target").unwrap(),
        expected_target: head,
    };
    assert!(
        capture(root, &binding(), &selection)
            .unwrap_err()
            .contains("merge driver")
    );
}

#[test]
fn symlink_mode_and_conflicted_merge_are_rejected_without_writing_index() {
    let dir = fixture("sha1");
    let root = dir.path();
    let base = oid(root, "HEAD");
    let blob = git(root, &["rev-parse", "HEAD:before.txt"]);
    git(
        root,
        &[
            "update-index",
            "--add",
            "--cacheinfo",
            &format!("120000,{blob},symbolic"),
        ],
    );
    let bytes = fs::read(root.join(".git/index")).unwrap();
    assert!(
        capture(root, &binding(), &Selection::Index { base: base.clone() })
            .unwrap_err()
            .contains("symlink")
    );
    assert_eq!(fs::read(root.join(".git/index")).unwrap(), bytes);
    git(root, &["update-index", "--force-remove", "symbolic"]);
    git(root, &["branch", "target"]);
    fs::write(root.join("before.txt"), "feature conflicting\n").unwrap();
    git(root, &["add", "."]);
    git(root, &["commit", "-m", "feature conflict"]);
    let head = oid(root, "HEAD");
    git(root, &["checkout", "target"]);
    fs::write(root.join("before.txt"), "target conflicting\n").unwrap();
    git(root, &["add", "."]);
    git(root, &["commit", "-m", "target conflict"]);
    let expected_target = oid(root, "HEAD");
    let bytes = fs::read(root.join(".git/index")).unwrap();
    let selection = Selection::MergeCandidate {
        head,
        target_ref: Identifier::new("refs/heads/target").unwrap(),
        expected_target,
    };
    assert!(capture(root, &binding(), &selection).is_err());
    assert_eq!(fs::read(root.join(".git/index")).unwrap(), bytes);
    assert_eq!(
        fs::read_to_string(root.join("before.txt")).unwrap(),
        "target conflicting\n"
    );
}

#[test]
fn sha256_merge_candidate_and_worktree_freshness_limits() {
    let dir = fixture("sha256");
    let root = dir.path();
    let base = oid(root, "HEAD");
    git(root, &["branch", "target"]);
    let selection = Selection::MergeCandidate {
        head: base.clone(),
        target_ref: Identifier::new("refs/heads/target").unwrap(),
        expected_target: base.clone(),
    };
    let candidate = capture(root, &binding(), &selection).unwrap();
    assert_eq!(
        candidate
            .snapshot
            .as_git()
            .unwrap()
            .tree
            .as_ref()
            .unwrap()
            .as_str()
            .len(),
        64
    );
    let selection = Selection::Worktree { base };
    let worktree = capture(root, &binding(), &selection).unwrap();
    fs::write(root.join("untracked.txt"), "new actual input").unwrap();
    assert!(verify_current(root, &binding(), &selection, &worktree).is_err());
    fs::write(root.join("untracked.txt"), vec![0; 4 * 1024 * 1024 + 1]).unwrap();
    assert!(
        capture(root, &binding(), &selection)
            .unwrap_err()
            .contains("oversized")
    );
}

#[test]
fn ignored_worktree_files_are_inputs_not_silently_excluded() {
    let dir = fixture("sha1");
    let root = dir.path();
    let base = oid(root, "HEAD");
    fs::write(root.join(".gitignore"), "local-config.txt\n").unwrap();
    fs::write(root.join("local-config.txt"), "original config\n").unwrap();
    let selection = Selection::Worktree { base };
    let previous = capture(root, &binding(), &selection).unwrap();
    assert!(
        previous
            .manifest
            .iter()
            .any(|entry| entry.path.to_bytes() == b"local-config.txt")
    );
    fs::write(root.join("local-config.txt"), "changed build input\n").unwrap();
    assert!(verify_current(root, &binding(), &selection, &previous).is_err());
}

#[test]
// Windows 分支仅清除私有 fixture 的只读属性；Unix 分支显式设置 0600，不扩大权限。
#[allow(clippy::permissions_set_readonly_false)]
fn replaced_loose_blob_tree_and_commit_objects_cannot_impersonate_git_oids() {
    for format in ["sha1", "sha256"] {
        for kind in ["blob", "tree", "commit"] {
            let dir = fixture(format);
            let root = dir.path();
            let original = oid(root, "HEAD");
            let suffix = match kind {
                "blob" => ":before.txt",
                "tree" => "^{tree}",
                _ => "",
            };
            let original_object = git(
                root,
                &["rev-parse", &format!("{}{suffix}", original.as_str())],
            );
            fs::write(root.join("before.txt"), "replacement content\n").unwrap();
            git(root, &["add", "-A"]);
            git(root, &["commit", "-m", "replacement"]);
            let replacement = git(root, &["rev-parse", &format!("HEAD{suffix}")]);
            let object_path = |value: &str| {
                root.join(".git/objects")
                    .join(&value[..2])
                    .join(&value[2..])
            };
            let target = object_path(&original_object);
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&target, fs::Permissions::from_mode(0o600)).unwrap();
            }
            #[cfg(windows)]
            {
                let mut permissions = fs::metadata(&target).unwrap().permissions();
                permissions.set_readonly(false);
                fs::set_permissions(&target, permissions).unwrap();
            }
            fs::copy(object_path(&replacement), &target).unwrap();
            let selection = Selection::Commit {
                commit: original.clone(),
                base: original,
            };
            assert!(
                capture(root, &binding(), &selection).is_err(),
                "accepted replaced {kind} in {format}"
            );
        }
    }
}

#[cfg(unix)]
#[test]
fn non_utf8_and_normalization_distinct_git_paths_remain_lossless() {
    use std::os::unix::ffi::OsStringExt;
    let dir = fixture("sha1");
    let root = dir.path();
    let base = oid(root, "HEAD");
    for raw in [
        b"raw-\xff\t\n.rs".to_vec(),
        "é.rs".as_bytes().to_vec(),
        "e\u{301}.rs".as_bytes().to_vec(),
    ] {
        fs::write(root.join(std::ffi::OsString::from_vec(raw)), "contents\n").unwrap();
    }
    git(root, &["add", "-A"]);
    let captured = capture(root, &binding(), &Selection::Index { base }).unwrap();
    assert_eq!(captured.manifest.len(), 4);
    assert!(
        captured
            .manifest
            .iter()
            .any(|entry| entry.path.to_bytes() == b"raw-\xff\t\n.rs")
    );
    assert!(
        captured
            .manifest
            .iter()
            .any(|entry| entry.path.to_bytes() == "é.rs".as_bytes())
    );
    assert!(
        captured
            .manifest
            .iter()
            .any(|entry| entry.path.to_bytes() == "e\u{301}.rs".as_bytes())
    );
}

#[cfg(windows)]
#[test]
fn worktree_case_aliases_are_rejected_but_git_object_paths_remain_distinct() {
    for format in ["sha1", "sha256"] {
        let dir = fixture(format);
        let root = dir.path();
        let base = oid(root, "HEAD");
        fs::write(root.join("item.txt"), "one actual file\n").unwrap();
        git(root, &["add", "item.txt"]);
        let blob = git(root, &["rev-parse", ":item.txt"]);
        git(
            root,
            &[
                "update-index",
                "--add",
                "--cacheinfo",
                &format!("100644,{blob},ITEM.txt"),
            ],
        );
        assert_eq!(
            fs::read_dir(root)
                .unwrap()
                .filter_map(Result::ok)
                .filter(|e| e.file_name() == "item.txt" || e.file_name() == "ITEM.txt")
                .count(),
            1
        );
        let error = capture(
            root,
            &binding(),
            &Selection::Worktree { base: base.clone() },
        )
        .unwrap_err();
        assert!(error.contains("worktree path alias"), "{error}");
        // 即使清单只留下错误大小写名称，也必须拒绝，不能仅靠双路径去重发现问题。
        git(root, &["update-index", "--force-remove", "item.txt"]);
        assert!(
            capture(
                root,
                &binding(),
                &Selection::Worktree { base: base.clone() }
            )
            .unwrap_err()
            .contains("worktree path alias")
        );
        git(
            root,
            &[
                "update-index",
                "--add",
                "--cacheinfo",
                &format!("100644,{blob},item.txt"),
            ],
        );
        let index = capture(root, &binding(), &Selection::Index { base: base.clone() }).unwrap();
        for path in [b"item.txt".as_slice(), b"ITEM.txt".as_slice()] {
            assert!(
                index
                    .manifest
                    .iter()
                    .any(|entry| entry.path.to_bytes() == path)
            );
        }
        git(root, &["commit", "-m", "preserve both Git names"]);
        let committed = capture(
            root,
            &binding(),
            &Selection::Commit {
                commit: oid(root, "HEAD"),
                base,
            },
        )
        .unwrap();
        assert_eq!(committed.manifest, index.manifest);
    }
}

#[test]
fn distinct_real_hard_link_names_remain_valid_worktree_inputs() {
    let dir = fixture("sha1");
    let root = dir.path();
    let base = oid(root, "HEAD");
    fs::hard_link(root.join("before.txt"), root.join("another.txt")).unwrap();
    let captured = capture(root, &binding(), &Selection::Worktree { base }).unwrap();
    assert_eq!(captured.manifest.len(), 2);
    for path in [b"before.txt".as_slice(), b"another.txt".as_slice()] {
        assert!(
            captured
                .manifest
                .iter()
                .any(|entry| entry.path.to_bytes() == path)
        );
    }
    assert_eq!(
        captured.manifest[0].content_digest,
        captured.manifest[1].content_digest
    );
}
