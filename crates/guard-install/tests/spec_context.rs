#![cfg(feature = "test-fixture")]
use serde_json::Value;
use std::{
    fs,
    path::Path,
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
fn project() -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!(
        "spec-adapter-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&path).unwrap();
    path
}
fn write(root: &Path, name: &str, text: &str) {
    let path = root.join(name);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}
fn call(root: &Path, args: &[&str]) -> Value {
    let output = Command::new(env!("CARGO_BIN_EXE_guard-install-fixture"))
        .args(args)
        .arg("--project")
        .arg(root)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}
#[test]
fn context_discovers_native_spec_without_creating_files() {
    let root = project();
    assert_eq!(
        call(&root, &["context"])["specification"]["status"],
        "unconfigured"
    );
    assert_eq!(fs::read_dir(&root).unwrap().count(), 0);
    for file in [
        "proposal.md",
        "design.md",
        "tasks.md",
        "specs/layers/spec.md",
    ] {
        write(
            &root,
            &format!("openspec/changes/layers/{file}"),
            "REQ-1 domain boundary",
        );
    }
    let result = call(&root, &["spec", "context"]);
    assert_eq!(result["status"], "ready");
    assert_eq!(result["system"], "openspec");
    assert_eq!(result["artifacts"].as_array().unwrap().len(), 4);
    let before = result["digest"].clone();
    write(
        &root,
        "openspec/changes/layers/specs/layers/spec.md",
        "REQ-1 revised",
    );
    assert_ne!(before, call(&root, &["spec", "context"])["digest"]);
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn ambiguous_systems_require_explicit_selection_and_target() {
    let root = project();
    for file in ["spec.md", "plan.md", "tasks.md"] {
        write(&root, &format!("specs/001-orders/{file}"), "REQ-1");
    }
    write(&root, ".specify/memory/constitution.md", "principles");
    write(&root, "openspec/config.yaml", "schema: spec-driven");
    assert_eq!(call(&root, &["spec", "context"])["status"], "ambiguous");
    let result = call(
        &root,
        &[
            "spec",
            "context",
            "--spec-system",
            "speckit",
            "--spec-target",
            "specs/001-orders",
        ],
    );
    assert_eq!(result["status"], "ready");
    assert_eq!(result["artifacts"].as_array().unwrap().len(), 4);
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn superpowers_and_missing_artifacts_are_explicit() {
    let root = project();
    write(&root, "docs/superpowers/specs/orders.md", "REQ-1");
    let result = call(&root, &["spec", "context"]);
    assert_eq!(result["system"], "superpowers");
    assert_eq!(result["status"], "ready");
    write(&root, "docs/superpowers/specs/payments.md", "REQ-2");
    assert_eq!(call(&root, &["spec", "context"])["status"], "ambiguous");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn incomplete_archived_and_unsafe_targets_never_become_ready() {
    let root = project();
    write(&root, "openspec/changes/archive/old/proposal.md", "old");
    write(&root, "openspec/changes/current/proposal.md", "REQ-1");
    let output = Command::new(env!("CARGO_BIN_EXE_guard-install-fixture"))
        .args(["spec", "context", "--project"])
        .arg(&root)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["status"], "incomplete");
    assert_eq!(value["target"], "openspec/changes/current");
    assert!(value["missing"].as_array().unwrap().len() >= 3);
    for target in [
        "../outside",
        "openspec/changes/archive",
        "openspec/changes/absent",
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_guard-install-fixture"))
            .args(["spec", "context", "--spec-target", target, "--project"])
            .arg(&root)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(4));
    }
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn file_size_and_directory_budgets_fail_closed() {
    let root = project();
    write(
        &root,
        "docs/superpowers/specs/large.md",
        &"x".repeat(1024 * 1024 + 1),
    );
    let output = Command::new(env!("CARGO_BIN_EXE_guard-install-fixture"))
        .args(["spec", "context", "--project"])
        .arg(&root)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(4));
    for i in 0..130 {
        fs::create_dir_all(root.join(format!("openspec/changes/budget/specs/empty-{i}"))).unwrap();
    }
    let output = Command::new(env!("CARGO_BIN_EXE_guard-install-fixture"))
        .args(["spec", "context", "--spec-system", "openspec", "--project"])
        .arg(&root)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(4));
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .contains("budget exceeded")
    );
    fs::remove_dir_all(root).unwrap();
}
