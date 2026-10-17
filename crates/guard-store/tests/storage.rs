use guard_store::{EvidenceStore, RecordState, StoreLimits, WritePhase};
use std::{
    fs,
    path::Path,
    process::Command,
    sync::{Arc, Barrier},
    thread,
};

fn fixture() -> (tempfile::TempDir, EvidenceStore) {
    let temp = tempfile::tempdir().unwrap();
    let store =
        EvidenceStore::create(&temp.path().join("private"), StoreLimits::default()).unwrap();
    (temp, store)
}

#[test]
fn private_round_trip_idempotency_and_public_redaction() {
    let (_temp, store) = fixture();
    let bytes = b"Authorization: Bearer secret-token\npassword=sensitive";
    let summary = store.put("op-1", bytes, 5000, 1000).unwrap();
    assert_eq!(store.read_private("op-1", 1001).unwrap(), bytes);
    assert_eq!(summary, store.put("op-1", bytes, 5000, 1002).unwrap());
    assert!(store.put("op-1", b"different", 5000, 1002).is_err());
    let public = serde_json::to_string(&summary).unwrap();
    for secret in ["secret-token", "password", "Authorization", "sensitive"] {
        assert!(!public.contains(secret));
    }
    assert_eq!(store.state("op-1").unwrap(), RecordState::Complete);
}

#[test]
fn missing_and_corrupt_artifacts_are_quarantined_not_returned() {
    let (temp, store) = fixture();
    store.put("missing", b"raw", 5000, 1000).unwrap();
    store.put("corrupt", b"good", 5000, 1000).unwrap();
    let missing = store.artifact_path("missing").unwrap();
    let corrupt = store.artifact_path("corrupt").unwrap();
    fs::remove_file(missing).unwrap();
    fs::write(corrupt, b"bad!").unwrap();
    assert!(store.read_private("missing", 1001).is_err());
    assert!(store.read_private("corrupt", 1001).is_err());
    drop(store);
    let reopened =
        EvidenceStore::open(&temp.path().join("private"), StoreLimits::default()).unwrap();
    assert_eq!(reopened.state("missing").unwrap(), RecordState::Quarantined);
    assert_eq!(reopened.state("corrupt").unwrap(), RecordState::Quarantined);
    assert!(reopened.audit_events().unwrap().len() >= 2);
}

#[test]
fn real_sqlite_busy_is_bounded_and_retry_does_not_duplicate() {
    let (temp, store) = fixture();
    let db = rusqlite::Connection::open(temp.path().join("private/evidence.sqlite3")).unwrap();
    db.execute_batch("BEGIN IMMEDIATE").unwrap();
    let start = std::time::Instant::now();
    assert!(store.put("busy", b"one", 5000, 1000).is_err());
    assert!(start.elapsed() < std::time::Duration::from_secs(3));
    db.execute_batch("ROLLBACK").unwrap();
    store.put("busy", b"one", 5000, 1000).unwrap();
    assert_eq!(store.read_private("busy", 1001).unwrap(), b"one");
}

#[test]
fn concurrent_connections_have_one_idempotent_result() {
    let (temp, store) = fixture();
    let root = temp.path().join("private");
    drop(store);
    // 先完成连接恢复准备，再同时竞争写入；默认有界锁允许竞争者返回busy。
    let stores: Vec<_> = (0..4)
        .map(|_| EvidenceStore::open(&root, StoreLimits::default()).unwrap())
        .collect();
    let barrier = Arc::new(Barrier::new(4));
    let handles: Vec<_> = stores
        .into_iter()
        .map(|store| {
            let barrier = barrier.clone();
            thread::spawn(move || {
                barrier.wait();
                let result = store.put("same", b"identical", 5000, 1000);
                (store, result)
            })
        })
        .collect();
    let attempts: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    assert!(attempts.iter().any(|(_, result)| result.is_ok()));
    // 并发阶段结束后只显式重试一次busy，保留生产300ms上限和其余错误。
    let values: Vec<_> = attempts
        .into_iter()
        .map(|(store, result)| match result {
            Ok(value) => value,
            Err(error) => {
                assert_eq!(error, "store busy");
                store.put("same", b"identical", 5000, 1000).unwrap()
            }
        })
        .collect();
    assert!(values.iter().all(|v| v == &values[0]));
    let store = EvidenceStore::open(&root, StoreLimits::default()).unwrap();
    assert_eq!(
        store
            .audit_events()
            .unwrap()
            .iter()
            .filter(|e| e.code == "completed")
            .count(),
        1
    );
}

#[test]
fn retention_expires_payload_but_keeps_audit_and_operation_identity() {
    let (_temp, store) = fixture();
    store.put("old", b"evidence", 1100, 1000).unwrap();
    assert_eq!(store.expire(1100).unwrap(), 1);
    assert_eq!(store.state("old").unwrap(), RecordState::Expired);
    assert!(store.read_private("old", 1101).is_err());
    assert!(store.put("old", b"evidence", 2200, 1200).is_err());
    assert!(!store.artifact_path("old").unwrap().exists());
    assert!(
        store
            .audit_events()
            .unwrap()
            .iter()
            .any(|event| event.code == "expired")
    );
}

#[test]
fn quotas_and_retention_bounds_are_fail_closed() {
    let temp = tempfile::tempdir().unwrap();
    let limits = StoreLimits {
        max_artifact_bytes: 4,
        max_total_bytes: 6,
        max_records: 2,
        max_retention_ms: 500,
        busy_timeout_ms: 100,
    };
    let store = EvidenceStore::create(&temp.path().join("private"), limits).unwrap();
    assert!(store.put("large", b"12345", 1100, 1000).is_err());
    assert!(store.put("forever", b"1", 1600, 1000).is_err());
    store.put("one", b"1234", 1100, 1000).unwrap();
    assert!(store.put("full", b"1234", 1100, 1000).is_err());
    store.put("two", b"12", 1100, 1000).unwrap();
    assert!(store.put("records", b"", 1100, 1000).is_err());
}

#[test]
fn abrupt_process_exit_after_durable_intent_or_artifact_preserves_recovery() {
    for phase in ["intent", "partial", "artifact"] {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("private");
        EvidenceStore::create(&root, StoreLimits::default()).unwrap();
        let status = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "crash_worker", "--nocapture"])
            .env("GUARD_STORE_TEST_ROOT", &root)
            .env("GUARD_STORE_TEST_PHASE", phase)
            .status()
            .unwrap();
        assert_eq!(status.code(), Some(71));
        let recovered = EvidenceStore::open(&root, StoreLimits::default()).unwrap();
        assert_eq!(
            recovered.state("interrupted").unwrap(),
            RecordState::Quarantined
        );
        assert!(recovered.read_private("interrupted", 1001).is_err());
        assert!(
            recovered
                .audit_events()
                .unwrap()
                .iter()
                .any(|e| e.code == "incomplete_recovered")
        );
        assert!(recovered.put("interrupted", b"data", 5000, 1000).is_err());
    }
}

#[test]
fn crash_worker() {
    let Some(root) = std::env::var_os("GUARD_STORE_TEST_ROOT") else {
        return;
    };
    let phase = std::env::var("GUARD_STORE_TEST_PHASE").unwrap();
    let store = EvidenceStore::open(Path::new(&root), StoreLimits::default()).unwrap();
    store
        .put_observed("interrupted", b"data", 5000, 1000, |event| {
            if phase == "partial" && event == WritePhase::IntentDurable {
                let connection =
                    rusqlite::Connection::open(Path::new(&root).join("evidence.sqlite3")).unwrap();
                let (operation, digest): (String, String) = connection
                    .query_row("SELECT operation,digest FROM records", [], |row| {
                        Ok((row.get(0)?, row.get(1)?))
                    })
                    .unwrap();
                let path = Path::new(&root).join("artifacts").join(format!(
                    "{}-{}.blob",
                    &operation[7..],
                    &digest[7..]
                ));
                let mut options = fs::OpenOptions::new();
                options.write(true).create_new(true);
                #[cfg(unix)]
                {
                    use std::os::unix::fs::OpenOptionsExt;
                    options.mode(0o600);
                }
                let mut file = options.open(path).unwrap();
                use std::io::Write;
                file.write_all(b"p").unwrap();
                file.sync_all().unwrap();
                std::process::exit(71);
            }
            if (phase == "intent" && event == WritePhase::IntentDurable)
                || (phase == "artifact" && event == WritePhase::ArtifactDurable)
            {
                std::process::exit(71);
            }
        })
        .unwrap();
    panic!("did not stop at requested phase");
}

fn make_link(link: &Path, target: &Path, directory: bool) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        let _ = directory;
        std::os::unix::fs::symlink(target, link)
    }
    #[cfg(windows)]
    {
        let result = if directory {
            std::os::windows::fs::symlink_dir(target, link)
        } else {
            std::os::windows::fs::symlink_file(target, link)
        };
        match result {
            Err(error) if error.raw_os_error() == Some(1314) && !directory => {
                fs::hard_link(target, link)
            }
            Err(error) if error.raw_os_error() == Some(1314) => junction(link, target),
            other => other,
        }
    }
}

#[test]
fn linked_database_wal_and_artifact_replacements_are_rejected() {
    let (temp, store) = fixture();
    store.put("op", b"data", 5000, 1000).unwrap();
    let outside = temp.path().join("outside");
    fs::write(&outside, b"data").unwrap();
    let artifact = store.artifact_path("op").unwrap();
    fs::remove_file(&artifact).unwrap();
    make_link(&artifact, &outside, false).unwrap();
    assert!(store.read_private("op", 1001).is_err());
    fs::remove_file(&artifact).unwrap();
    drop(store);
    let wal = temp.path().join("private/evidence.sqlite3-wal");
    if wal.exists() {
        fs::remove_file(&wal).unwrap();
    }
    make_link(&wal, &outside, false).unwrap();
    assert!(EvidenceStore::open(&temp.path().join("private"), StoreLimits::default()).is_err());
}

#[test]
fn database_corruption_is_not_replaced_or_reinitialized() {
    let (temp, store) = fixture();
    drop(store);
    let database = temp.path().join("private/evidence.sqlite3");
    fs::write(&database, b"not a sqlite database").unwrap();
    assert!(EvidenceStore::open(&temp.path().join("private"), StoreLimits::default()).is_err());
    assert_eq!(fs::read(database).unwrap(), b"not a sqlite database");
}

#[test]
fn hard_linked_artifact_and_database_are_rejected() {
    let (temp, store) = fixture();
    store.put("op", b"data", 5000, 1000).unwrap();
    fs::hard_link(
        store.artifact_path("op").unwrap(),
        temp.path().join("alias"),
    )
    .unwrap();
    assert!(
        store.read_private("op", 1001).is_err(),
        "hardlinked private artifact was accepted"
    );
    drop(store);
    fs::hard_link(
        temp.path().join("private/evidence.sqlite3"),
        temp.path().join("database-alias"),
    )
    .unwrap();
    assert!(EvidenceStore::open(&temp.path().join("private"), StoreLimits::default()).is_err());
}

#[test]
fn ordinary_lock_file_replacement_cannot_split_writer_ownership() {
    let (temp, store) = fixture();
    let root = temp.path().join("private");
    let result = store.put_observed("first", b"data", 5000, 1000, |phase| {
        if phase != WritePhase::IntentDurable {
            return;
        }
        fs::rename(root.join("writer.lock"), root.join("writer.old")).unwrap();
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        options.open(root.join("writer.lock")).unwrap();
        assert!(
            EvidenceStore::open(&root, StoreLimits::default()).is_err(),
            "replacement writer lock was trusted"
        );
    });
    assert!(result.is_err());
}

#[test]
fn linked_wal_sidecar_is_rejected_before_sqlite_opens_it() {
    let (temp, store) = fixture();
    drop(store);
    let outside = temp.path().join("outside");
    fs::write(&outside, b"do-not-touch").unwrap();
    let wal = temp.path().join("private/evidence.sqlite3-wal");
    fs::hard_link(&outside, &wal).unwrap();
    assert!(EvidenceStore::open(&temp.path().join("private"), StoreLimits::default()).is_err());
    assert_eq!(fs::read(outside).unwrap(), b"do-not-touch");
}

#[test]
fn sqlite_uncommitted_transaction_recovers_without_inventing_completion() {
    let (temp, store) = fixture();
    store.put("existing", b"safe", 5000, 1000).unwrap();
    let status = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "sqlite_crash_worker", "--nocapture"])
        .env("GUARD_STORE_TEST_ROOT", temp.path().join("private"))
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(72));
    let recovered =
        EvidenceStore::open(&temp.path().join("private"), StoreLimits::default()).unwrap();
    assert_eq!(recovered.state("existing").unwrap(), RecordState::Complete);
    assert_eq!(recovered.read_private("existing", 1001).unwrap(), b"safe");
}

#[test]
fn sqlite_crash_worker() {
    let Some(root) = std::env::var_os("GUARD_STORE_TEST_ROOT") else {
        return;
    };
    let connection = rusqlite::Connection::open(Path::new(&root).join("evidence.sqlite3")).unwrap();
    connection
        .execute_batch("BEGIN IMMEDIATE; UPDATE records SET state='expired';")
        .unwrap();
    std::process::exit(72);
}

#[cfg(windows)]
#[test]
fn windows_junction_replacement_is_rejected_without_symlink_privilege() {
    let (temp, store) = fixture();
    let managed = temp.path().join("private/artifacts");
    let moved = temp.path().join("moved");
    fs::rename(&managed, &moved).unwrap();
    junction(&managed, &moved).unwrap();
    assert!(store.put("escape", b"data", 5000, 1000).is_err());
    fs::remove_dir(managed).unwrap();
}

#[cfg(windows)]
fn junction(link: &Path, target: &Path) -> std::io::Result<()> {
    let status = Command::new("C:/Windows/System32/WindowsPowerShell/v1.0/powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command",
            "$ErrorActionPreference='Stop'; New-Item -ItemType Junction -Path $env:GUARD_TEST_LINK -Target $env:GUARD_TEST_TARGET | Out-Null"])
        .env("GUARD_TEST_LINK", link.to_string_lossy().replace('/', "\\").replace(r"\\?\", ""))
        .env("GUARD_TEST_TARGET", target.to_string_lossy().replace('/', "\\").replace(r"\\?\", ""))
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(std::io::Error::other("junction creation failed"))
    }
}

#[test]
fn managed_directory_replacement_is_rejected() {
    let (temp, store) = fixture();
    let root = temp.path().join("private");
    let old = temp.path().join("moved");
    fs::rename(root.join("artifacts"), &old).unwrap();
    make_link(&root.join("artifacts"), &old, true).unwrap();
    assert!(store.put("escape", b"data", 5000, 1000).is_err());
}
