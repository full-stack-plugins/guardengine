use guardengine::integration::{EvidenceProfile, GuardRunEnvelope, stage_attempt};
use std::fs;
fn envelope() -> GuardRunEnvelope {
    serde_json::from_slice(include_bytes!(
        "fixtures/integration-envelope/valid-native.json"
    ))
    .unwrap()
}
#[test]
fn interrupted_publish_is_not_current_and_existing_attempt_is_immutable() {
    let root = tempfile::tempdir().unwrap();
    let staged = stage_attempt(root.path(), &envelope(), EvidenceProfile::NativeOnly).unwrap();
    assert!(!staged.destination().exists());
    let destination = staged.destination().to_path_buf();
    drop(staged);
    assert!(!destination.exists());
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
    let published = stage_attempt(root.path(), &envelope(), EvidenceProfile::NativeOnly)
        .unwrap()
        .publish()
        .unwrap();
    let original = fs::read(&published.path).unwrap();
    assert_eq!(
        serde_json::from_slice::<GuardRunEnvelope>(&original).unwrap(),
        envelope()
    );
    assert!(
        stage_attempt(root.path(), &envelope(), EvidenceProfile::NativeOnly)
            .unwrap()
            .publish()
            .is_err()
    );
    assert_eq!(fs::read(&published.path).unwrap(), original);
}
#[test]
fn publication_rejects_invalid_data_and_does_not_interpret_run_id_as_path() {
    let root = tempfile::tempdir().unwrap();
    let victim = root
        .path()
        .parent()
        .unwrap()
        .join(format!("guard-victim-{}", std::process::id()));
    fs::write(&victim, b"source").unwrap();
    let mut e = envelope();
    e.run_id = "../source".into();
    let published = stage_attempt(root.path(), &e, EvidenceProfile::NativeOnly)
        .unwrap()
        .publish()
        .unwrap();
    assert_eq!(published.path.parent().unwrap(), root.path());
    assert_eq!(fs::read(&victim).unwrap(), b"source");
    fs::remove_file(&victim).unwrap();
    let mut e = envelope();
    e.binding.candidate_oid = "main".into();
    let count = fs::read_dir(root.path()).unwrap().count();
    assert!(stage_attempt(root.path(), &e, EvidenceProfile::NativeOnly).is_err());
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), count);
}
#[cfg(unix)]
#[test]
fn publication_rejects_symlink_and_public_storage_roots() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let outer = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    let alias = outer.path().join("alias");
    symlink(root.path(), &alias).unwrap();
    assert!(stage_attempt(&alias, &envelope(), EvidenceProfile::NativeOnly).is_err());
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o777)).unwrap();
    assert!(stage_attempt(root.path(), &envelope(), EvidenceProfile::NativeOnly).is_err());
}
#[cfg(unix)]
#[test]
fn cancelled_stage_does_not_unlink_a_replacement_storage_directory_entry() {
    use std::os::unix::fs::PermissionsExt;
    let parent = tempfile::tempdir().unwrap();
    let root = parent.path().join("store");
    fs::create_dir(&root).unwrap();
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
    let staged = stage_attempt(&root, &envelope(), EvidenceProfile::NativeOnly).unwrap();
    let name = fs::read_dir(&root)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .file_name();
    fs::rename(&root, parent.path().join("old-store")).unwrap();
    fs::create_dir(&root).unwrap();
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
    let replacement = root.join(name);
    fs::write(&replacement, b"unrelated data").unwrap();
    drop(staged);
    assert_eq!(fs::read(&replacement).unwrap(), b"unrelated data");
}
