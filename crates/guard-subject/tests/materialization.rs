use guard_protocol::{Digest, GitOid, Identifier};
use guard_subject::{CaptureBinding, Selection, capture, materialize};
use std::{
    fs,
    io::Write,
    path::Path,
    process::{Command, Stdio},
};
fn git(root: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
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
        out.status.success(),
        "{:?}: {}",
        args,
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap().trim().into()
}
fn binding() -> CaptureBinding {
    let d = Digest::new(format!("sha256:{}", "a".repeat(64))).unwrap();
    CaptureBinding {
        repository_id: Identifier::new("repo").unwrap(),
        task_id: Identifier::new("task").unwrap(),
        worktree_id: Identifier::new("worktree").unwrap(),
        contract_digest: d.clone(),
        policy_digest: d.clone(),
        verifier_digest: d.clone(),
        environment_digest: d,
    }
}
fn fixture(format: &str) -> tempfile::TempDir {
    let tmp = tempfile::tempdir().unwrap();
    let r = tmp.path();
    git(r, &["init", &format!("--object-format={format}")]);
    git(r, &["config", "user.name", "Materialize"]);
    git(r, &["config", "user.email", "fixture@example.invalid"]);
    git(r, &["config", "core.autocrlf", "false"]);
    fs::create_dir(r.join("src")).unwrap();
    fs::write(r.join("src/Example.java"), b"one\ntwo\nthree\n").unwrap();
    git(r, &["add", "."]);
    git(r, &["commit", "-m", "base"]);
    tmp
}
fn oid(root: &Path, name: &str) -> GitOid {
    GitOid::new(git(root, &["rev-parse", name])).unwrap()
}
fn objects(root: &Path) -> Vec<(std::path::PathBuf, Vec<u8>)> {
    fn visit(p: &Path, v: &mut Vec<(std::path::PathBuf, Vec<u8>)>) {
        for item in fs::read_dir(p).unwrap() {
            let p = item.unwrap().path();
            if p.is_dir() {
                visit(&p, v)
            } else {
                v.push((p.clone(), fs::read(p).unwrap()));
            }
        }
    }
    let mut result = Vec::new();
    visit(&root.join(".git/objects"), &mut result);
    result.sort();
    result
}

#[test]
fn all_four_candidates_materialize_their_real_bytes_without_writing_original_git() {
    for format in ["sha1", "sha256"] {
        let tmp = fixture(format);
        let root = tmp.path();
        let scratch = tempfile::tempdir().unwrap();
        let b = binding();
        let base = oid(root, "HEAD");
        fs::write(root.join("src/Example.java"), b"STAGED\n").unwrap();
        git(root, &["add", "."]);
        fs::write(root.join("src/Example.java"), b"DIRTY\n").unwrap();
        for (selection, expected) in [
            (
                Selection::Commit {
                    commit: base.clone(),
                    base: base.clone(),
                },
                b"one\ntwo\nthree\n".as_slice(),
            ),
            (
                Selection::Index { base: base.clone() },
                b"STAGED\n".as_slice(),
            ),
            (
                Selection::Worktree { base: base.clone() },
                b"DIRTY\n".as_slice(),
            ),
        ] {
            let subject = capture(root, &b, &selection).unwrap();
            let old_index = fs::read(root.join(".git/index")).unwrap();
            let old_objects = objects(root);
            let old_head = oid(root, "HEAD");
            let old_refs = git(root, &["show-ref"]);
            let material = materialize(root, &b, &selection, &subject, scratch.path()).unwrap();
            assert_eq!(material.captured(), &subject);
            assert_eq!(
                fs::read(material.path().join("src/Example.java")).unwrap(),
                expected
            );
            material.verify_inputs().unwrap();
            assert_eq!(fs::read(root.join(".git/index")).unwrap(), old_index);
            assert_eq!(objects(root), old_objects);
            assert_eq!(oid(root, "HEAD"), old_head);
            assert_eq!(git(root, &["show-ref"]), old_refs);
            let path = material.path().to_path_buf();
            drop(material);
            assert!(!path.exists());
        }
        git(root, &["reset", "--hard", "HEAD"]);
        git(root, &["branch", "target"]);
        fs::write(root.join("src/Example.java"), b"FEATURE\ntwo\nthree\n").unwrap();
        git(root, &["add", "."]);
        git(root, &["commit", "-m", "feature"]);
        let head = oid(root, "HEAD");
        git(root, &["checkout", "target"]);
        fs::write(root.join("src/Example.java"), b"one\ntwo\nTARGET\n").unwrap();
        git(root, &["add", "."]);
        git(root, &["commit", "-m", "target"]);
        let expected_target = oid(root, "HEAD");
        let selection = Selection::MergeCandidate {
            head,
            target_ref: Identifier::new("refs/heads/target").unwrap(),
            expected_target,
        };
        let subject = capture(root, &b, &selection).unwrap();
        let merged_blob = subject.manifest[0].object_id.as_ref().unwrap();
        let exists_in_original = Command::new("git")
            .arg("-C")
            .arg(root)
            .args(["cat-file", "-e", merged_blob.as_str()])
            .env("GIT_NO_LAZY_FETCH", "1")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap();
        assert!(
            !exists_in_original.success(),
            "merged blob must only exist in private temporary objects"
        );
        fs::write(root.join("src/Example.java"), b"DIRTY MERGE WORKTREE").unwrap();
        let old_index = fs::read(root.join(".git/index")).unwrap();
        let old_objects = objects(root);
        let old_head = oid(root, "HEAD");
        let old_refs = git(root, &["show-ref"]);
        let material = materialize(root, &b, &selection, &subject, scratch.path()).unwrap();
        assert_eq!(
            fs::read(material.path().join("src/Example.java")).unwrap(),
            b"FEATURE\ntwo\nTARGET\n"
        );
        assert_eq!(material.captured(), &subject);
        assert_eq!(objects(root), old_objects);
        assert_eq!(fs::read(root.join(".git/index")).unwrap(), old_index);
        assert_eq!(oid(root, "HEAD"), old_head);
        assert_eq!(git(root, &["show-ref"]), old_refs);
    }
}

#[test]
fn verification_checks_frozen_inputs_while_allowing_new_build_outputs() {
    let tmp = fixture("sha1");
    let root = tmp.path();
    let scratch = tempfile::tempdir().unwrap();
    let b = binding();
    let selection = Selection::Worktree {
        base: oid(root, "HEAD"),
    };
    let subject = capture(root, &b, &selection).unwrap();
    let material = materialize(root, &b, &selection, &subject, scratch.path()).unwrap();
    fs::create_dir(material.path().join("target")).unwrap();
    fs::write(
        material.path().join("target/generated.java"),
        b"build output",
    )
    .unwrap();
    material.verify_inputs().unwrap();
    assert_eq!(material.captured().manifest.len(), 1);
    fs::write(material.path().join("src/Example.java"), b"changed").unwrap();
    assert!(material.verify_inputs().is_err());
    fs::remove_file(material.path().join("src/Example.java")).unwrap();
    assert!(material.verify_inputs().is_err());
    assert!(materialize(root, &b, &selection, &subject, root).is_err());
    assert!(materialize(root, &b, &selection, &subject, &root.join(".git/objects")).is_err());
    fs::write(root.join("src/Example.java"), b"source changed").unwrap();
    assert!(materialize(root, &b, &selection, &subject, scratch.path()).is_err());
}

fn git_input(root: &Path, args: &[&str], bytes: &[u8]) -> String {
    let mut child = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(bytes).unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap().trim().into()
}

#[test]
fn handcrafted_git_control_files_and_aliases_are_never_materialized() {
    let tmp = fixture("sha1");
    let root = tmp.path();
    let scratch = tempfile::tempdir().unwrap();
    let b = binding();
    let base = oid(root, "HEAD");
    let blob = git_input(
        root,
        &["hash-object", "-w", "--stdin"],
        format!("gitdir: {}\n", root.join(".git").display()).as_bytes(),
    );
    for name in [".git", ".GIT", ".git. ", "git~1"] {
        let mut raw = format!("100644 {name}\0").into_bytes();
        for pos in (0..blob.len()).step_by(2) {
            raw.push(u8::from_str_radix(&blob[pos..pos + 2], 16).unwrap());
        }
        let tree = git_input(
            root,
            &["hash-object", "-t", "tree", "--literally", "-w", "--stdin"],
            &raw,
        );
        let commit = GitOid::new(git(
            root,
            &[
                "commit-tree",
                &tree,
                "-p",
                base.as_str(),
                "-m",
                "control object",
            ],
        ))
        .unwrap();
        let selection = Selection::Commit {
            commit,
            base: base.clone(),
        };
        let subject = capture(root, &b, &selection).unwrap();
        let before = objects(root);
        let error = materialize(root, &b, &selection, &subject, scratch.path())
            .err()
            .expect("Git control input rejected");
        assert!(error.contains("Git control path"), "{name}: {error}");
        assert_eq!(objects(root), before);
    }
}

#[test]
fn external_linked_worktree_git_metadata_is_rejected_before_scratch_writes() {
    let original = fixture("sha1");
    let container = tempfile::tempdir().unwrap();
    let root = container.path().join("linked");
    git(
        original.path(),
        &[
            "worktree",
            "add",
            "-b",
            "linked",
            root.to_str().unwrap(),
            "HEAD",
        ],
    );
    let root = root.canonicalize().unwrap();
    let b = binding();
    let selection = Selection::Worktree {
        base: oid(&root, "HEAD"),
    };
    let subject = capture(&root, &b, &selection).unwrap();
    for name in ["--git-common-dir", "--absolute-git-dir"] {
        let metadata = root.join(git(&root, &["rev-parse", name]));
        let before: Vec<_> = fs::read_dir(&metadata)
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert!(materialize(&root, &b, &selection, &subject, &metadata).is_err());
        let after: Vec<_> = fs::read_dir(&metadata)
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(before, after);
    }
}

#[test]
fn executable_mode_is_preserved_and_copies_do_not_share_original_file_writes() {
    let tmp = fixture("sha1");
    let root = tmp.path();
    let scratch = tempfile::tempdir().unwrap();
    git(root, &["update-index", "--chmod=+x", "src/Example.java"]);
    let b = binding();
    let selection = Selection::Index {
        base: oid(root, "HEAD"),
    };
    let subject = capture(root, &b, &selection).unwrap();
    let material = materialize(root, &b, &selection, &subject, scratch.path()).unwrap();
    assert_eq!(material.captured().manifest[0].mode, "100755");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let path = material.path().join("src/Example.java");
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o755
        );
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
        assert!(material.verify_inputs().is_err());
    }
    fs::write(
        material.path().join("src/Example.java"),
        b"only private copy",
    )
    .unwrap();
    assert_eq!(
        fs::read(root.join("src/Example.java")).unwrap(),
        b"one\ntwo\nthree\n"
    );
}

#[cfg(windows)]
#[test]
fn windows_git_case_aliases_cannot_be_written_or_reused_as_materialized_inputs() {
    let tmp = fixture("sha1");
    let root = tmp.path();
    let scratch = tempfile::tempdir().unwrap();
    let b = binding();
    let base = oid(root, "HEAD");
    let blob = git(root, &["rev-parse", ":src/Example.java"]);
    git(
        root,
        &[
            "update-index",
            "--add",
            "--cacheinfo",
            &format!("100644,{blob},src/EXAMPLE.java"),
        ],
    );
    let selection = Selection::Index { base: base.clone() };
    let subject = capture(root, &b, &selection).unwrap();
    assert!(materialize(root, &b, &selection, &subject, scratch.path()).is_err());
    git(
        root,
        &["update-index", "--force-remove", "src/EXAMPLE.java"],
    );
    let selection = Selection::Index { base };
    let subject = capture(root, &b, &selection).unwrap();
    let material = materialize(root, &b, &selection, &subject, scratch.path()).unwrap();
    let path = material.path().join("src/Example.java");
    let intermediate = material.path().join("src/rename-temp");
    fs::rename(&path, &intermediate).unwrap();
    fs::rename(&intermediate, material.path().join("src/EXAMPLE.java")).unwrap();
    assert!(material.verify_inputs().is_err());
}

#[test]
fn linked_parent_is_rejected_and_materialization_is_send_sync() {
    fn assert_thread_safe<T: Send + Sync>() {}
    assert_thread_safe::<guard_subject::MaterializedSubject>();
    let tmp = fixture("sha1");
    let scratch = tempfile::tempdir().unwrap();
    let container = tempfile::tempdir().unwrap();
    let alias = container.path().join("linked");
    #[cfg(unix)]
    std::os::unix::fs::symlink(scratch.path(), &alias).unwrap();
    #[cfg(windows)]
    {
        let output=Command::new("C:/Windows/System32/WindowsPowerShell/v1.0/powershell.exe").args(["-NoProfile","-NonInteractive","-Command","New-Item -ItemType Junction -Path $env:GUARD_MATERIALIZE_ALIAS -Target $env:GUARD_MATERIALIZE_TARGET | Out-Null"]).env("GUARD_MATERIALIZE_ALIAS",&alias).env("GUARD_MATERIALIZE_TARGET",scratch.path()).output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let b = binding();
    let selection = Selection::Worktree {
        base: oid(tmp.path(), "HEAD"),
    };
    let subject = capture(tmp.path(), &b, &selection).unwrap();
    assert!(materialize(tmp.path(), &b, &selection, &subject, &alias).is_err());
    assert_eq!(fs::read_dir(scratch.path()).unwrap().count(), 0);
}
