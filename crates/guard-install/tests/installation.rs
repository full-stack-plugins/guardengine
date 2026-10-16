#![cfg(feature = "test-fixture")]

use std::{
    fs,
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
fn root() -> PathBuf {
    let p = std::env::temp_dir().join(format!(
        "guard-install-test-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&p).unwrap();
    p
}
fn run(root: &std::path::Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_guard-install-fixture"))
        .args(args)
        .arg("--project")
        .arg(root)
        .output()
        .unwrap()
}
#[test]
fn install_is_idempotent_and_uninstall_preserves_human_content() {
    let p = root();
    fs::write(p.join("AGENTS.md"), "# Human\r\nkeep exactly\r\n").unwrap();
    let first = run(&p, &["install", "--agents", "codex"]);
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stdout)
    );
    let installed = fs::read(p.join("AGENTS.md")).unwrap();
    assert!(run(&p, &["install", "--agents", "codex"]).status.success());
    assert_eq!(installed, fs::read(p.join("AGENTS.md")).unwrap());
    assert!(run(&p, &["uninstall"]).status.success());
    assert_eq!(
        fs::read_to_string(p.join("AGENTS.md")).unwrap(),
        "# Human\r\nkeep exactly\r\n"
    );
    fs::remove_dir_all(p).unwrap();
}
#[test]
fn preview_writes_nothing() {
    let p = root();
    let result = run(
        &p,
        &["install", "--agents", "codex,claude,gemini", "--dry-run"],
    );
    assert!(result.status.success());
    assert_eq!(fs::read_dir(&p).unwrap().count(), 0);
    fs::remove_dir_all(p).unwrap();
}
