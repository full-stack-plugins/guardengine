use guard_protocol::Digest;
use guard_subject::{GitProvider, configure_git_provider};
use sha2::{Digest as ShaDigest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

fn digest(path: &Path) -> Digest {
    Digest::new(format!(
        "sha256:{:x}",
        Sha256::digest(fs::read(path).unwrap())
    ))
    .unwrap()
}
fn real_git() -> PathBuf {
    let name = if cfg!(windows) { "git.exe" } else { "git" };
    std::env::split_paths(&std::env::var_os("PATH").unwrap())
        .map(|p| p.join(name))
        .find(|p| p.is_file())
        .unwrap()
        .canonicalize()
        .unwrap()
}
fn provider(executable: PathBuf) -> GitProvider {
    let mut environment = BTreeMap::new();
    for key in ["SystemRoot", "WINDIR", "TEMP", "TMP"] {
        if let Ok(value) = std::env::var(key) {
            environment.insert(key.into(), value);
        }
    }
    GitProvider {
        artifact_digest: digest(&executable),
        executable,
        environment,
    }
}
fn isolated(mode: &str) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    let git = real_git();
    let output = Command::new(&git).arg("init").arg(&root).output().unwrap();
    assert!(output.status.success());
    fs::write(root.join("tracked.txt"), "real input\n").unwrap();
    for args in [
        vec!["add", "tracked.txt"],
        vec![
            "-c",
            "user.name=Provider",
            "-c",
            "user.email=provider@example.invalid",
            "commit",
            "-m",
            "initial",
        ],
    ] {
        assert!(
            Command::new(&git)
                .arg("-C")
                .arg(&root)
                .args(args)
                .output()
                .unwrap()
                .status
                .success()
        );
    }
    let head = Command::new(&git)
        .arg("-C")
        .arg(&root)
        .args(["rev-parse", "HEAD"])
        .output()
        .unwrap();
    let base = String::from_utf8(head.stdout).unwrap().trim().to_owned();
    let fake = root.join(if cfg!(windows) { "git.exe" } else { "git" });
    fs::copy(std::env::current_exe().unwrap(), &fake).unwrap();
    let result = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "provider_worker", "--nocapture"])
        .current_dir(&root)
        .env("PATH", &root)
        .env("GIT_PROVIDER_MODE", mode)
        .env("GIT_PROVIDER_REAL", git)
        .env("GIT_PROVIDER_ROOT", root)
        .env("GIT_PROVIDER_BASE", base)
        .env("SUBJECT_PROVIDER_POISON", "must not reach provider")
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{mode}: {} {}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
}
#[test]
fn absolute_provider_ignores_fake_path_and_cwd() {
    isolated("fixed");
}
#[test]
fn default_sdk_use_prevents_late_configuration() {
    isolated("default");
}
#[test]
fn configuration_is_immutable_and_exactly_idempotent() {
    isolated("immutable");
}
#[test]
fn invalid_pin_and_environment_are_rejected() {
    isolated("invalid");
}
#[test]
fn changed_pinned_executable_is_rejected_before_spawn() {
    isolated("changed");
}

#[test]
fn provider_worker() {
    let Ok(mode) = std::env::var("GIT_PROVIDER_MODE") else {
        return;
    };
    let executable = PathBuf::from(std::env::var_os("GIT_PROVIDER_REAL").unwrap());
    let root = PathBuf::from(std::env::var_os("GIT_PROVIDER_ROOT").unwrap());
    let configured = provider(executable);
    #[cfg(feature = "git-provider-test-fixture")]
    if mode == "clear" || mode == "after" {
        let fixture = PathBuf::from(env!("CARGO_BIN_EXE_git-provider-fixture"));
        let copy = root.join(if cfg!(windows) {
            "observer.exe"
        } else {
            "observer"
        });
        fs::copy(&fixture, &copy).unwrap();
        let mut p = provider(copy);
        p.environment.insert(
            "SUBJECT_PROVIDER_REAL_GIT".into(),
            configured.executable.to_str().unwrap().into(),
        );
        if mode == "after" {
            p.environment.insert(
                "SUBJECT_PROVIDER_MUTATOR".into(),
                fixture.to_str().unwrap().into(),
            );
        }
        configure_git_provider(p).unwrap();
        let result = guard_subject::capture(
            &root,
            &binding(),
            &guard_subject::Selection::Index {
                base: guard_protocol::GitOid::new(std::env::var("GIT_PROVIDER_BASE").unwrap())
                    .unwrap(),
            },
        );
        if mode == "after" {
            assert!(result.unwrap_err().contains("digest"));
        } else {
            result.unwrap();
        }
        return;
    }
    if mode == "default" {
        // 捕获入口实际尝试默认 Git，失败同样冻结默认 provider。
        let _ = guard_subject::capture(
            &root,
            &binding(),
            &guard_subject::Selection::Index {
                base: guard_protocol::GitOid::new("0".repeat(40)).unwrap(),
            },
        );
        assert!(
            configure_git_provider(configured)
                .unwrap_err()
                .contains("already")
        );
        return;
    }
    if mode == "invalid" {
        let mut bad = configured.clone();
        bad.artifact_digest = Digest::new(format!("sha256:{}", "0".repeat(64))).unwrap();
        assert!(configure_git_provider(bad).is_err());
        let mut bad = configured.clone();
        bad.environment
            .insert("GIT_CONFIG_COUNT".into(), "1".into());
        assert!(configure_git_provider(bad).is_err());
        let mut bad = configured.clone();
        bad.environment.insert("Path".into(), "a".into());
        bad.environment.insert("PATH".into(), "b".into());
        assert!(configure_git_provider(bad).is_err());
        let mut bad = configured.clone();
        bad.environment.insert("BAD=KEY".into(), "a".into());
        assert!(configure_git_provider(bad).is_err());
        let mut bad = configured.clone();
        bad.environment.insert("EMPTY".into(), "x".repeat(32769));
        assert!(configure_git_provider(bad).is_err());
        let mut bad = configured.clone();
        bad.environment
            .extend((0..257).map(|i| (format!("KEY_{i}"), "value".into())));
        assert!(configure_git_provider(bad).is_err());
        let mut bad = configured.clone();
        bad.environment
            .extend((0..5).map(|i| (format!("KEY_{i}"), "v".repeat(32768))));
        assert!(configure_git_provider(bad).is_err());
        let mut bad = configured.clone();
        bad.environment.insert("VALUE".into(), "a\0b".into());
        assert!(configure_git_provider(bad).is_err());
        let mut bad = configured.clone();
        bad.executable = root.clone();
        assert!(configure_git_provider(bad).is_err());
        let mut bad = configured.clone();
        bad.executable = PathBuf::from("git");
        assert!(configure_git_provider(bad).is_err());
    }
    if mode == "changed" {
        let path = root.join(if cfg!(windows) {
            "pinned.exe"
        } else {
            "pinned"
        });
        fs::copy(&configured.executable, &path).unwrap();
        configure_git_provider(provider(path.clone())).unwrap();
        fs::write(path, b"changed executable").unwrap();
        let result = guard_subject::capture(
            &root,
            &binding(),
            &guard_subject::Selection::Index {
                base: guard_protocol::GitOid::new("0".repeat(40)).unwrap(),
            },
        );
        assert!(result.unwrap_err().contains("digest"));
        return;
    }
    configure_git_provider(configured.clone()).unwrap();
    configure_git_provider(configured.clone()).unwrap();
    if mode == "immutable" {
        let mut changed = configured;
        changed.environment.insert("OTHER".into(), "value".into());
        assert!(
            configure_git_provider(changed)
                .unwrap_err()
                .contains("already")
        );
    }
    if mode == "fixed" {
        let selection = guard_subject::Selection::Index {
            base: guard_protocol::GitOid::new(std::env::var("GIT_PROVIDER_BASE").unwrap()).unwrap(),
        };
        let captured = guard_subject::capture(&root, &binding(), &selection).unwrap();
        guard_subject::verify_current(&root, &binding(), &selection, &captured).unwrap();
        return;
    }
    // 真正 Git 将拒绝不存在对象；假 Git 的测试程序会拒绝 plumbing argv。
    let error = guard_subject::capture(
        &root,
        &binding(),
        &guard_subject::Selection::Index {
            base: guard_protocol::GitOid::new("0".repeat(40)).unwrap(),
        },
    )
    .unwrap_err();
    assert!(
        error.contains("Git")
            && !error.contains("unexpected argument")
            && !error.contains("unrecognized option"),
        "{error}"
    );
    assert!(
        error.contains("cat-file") || error.contains("rev-parse") || error.contains("object"),
        "{error}"
    );
}
fn binding() -> guard_subject::CaptureBinding {
    let d = Digest::new(format!("sha256:{}", "a".repeat(64))).unwrap();
    guard_subject::CaptureBinding {
        repository_id: guard_protocol::Identifier::new("test-repo").unwrap(),
        task_id: guard_protocol::Identifier::new("test-task").unwrap(),
        worktree_id: guard_protocol::Identifier::new("test-worktree").unwrap(),
        contract_digest: d.clone(),
        policy_digest: d.clone(),
        verifier_digest: d.clone(),
        environment_digest: d,
    }
}

#[cfg(feature = "git-provider-test-fixture")]
#[test]
fn fixed_provider_clears_unapproved_inherited_environment() {
    isolated("clear");
}
#[cfg(feature = "git-provider-test-fixture")]
#[test]
fn replacement_after_command_exit_is_rejected() {
    isolated("after");
}
