use guard_store::{EvidenceStore, HeadUpdate, StoreLimits};
use std::{
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
fn ordinary_put_is_not_an_active_head_and_cas_survives_restart() {
    let (temp, store) = fixture();
    store.put("first", b"private record", 5000, 1000).unwrap();
    assert!(store.read_head("task", 1001).unwrap().is_none());
    let HeadUpdate::Applied(identity) = store
        .compare_and_swap_head("task", None, "first", 1001)
        .unwrap()
    else {
        panic!("first CAS")
    };
    let current = store.read_head("task", 1002).unwrap().unwrap();
    assert_eq!(current.identity, identity);
    assert_eq!(current.bytes, b"private record");
    assert!(matches!(
        store
            .compare_and_swap_head("task", None, "first", 1002)
            .unwrap(),
        HeadUpdate::Unchanged(_)
    ));
    drop(store);
    let store = EvidenceStore::open(&temp.path().join("private"), StoreLimits::default()).unwrap();
    assert_eq!(
        store.read_head("task", 1003).unwrap().unwrap().identity,
        identity
    );
}

#[test]
fn concurrent_threads_have_one_winner_and_stale_cas_does_not_replace_head() {
    let (temp, store) = fixture();
    for op in ["one", "two", "three"] {
        store.put(op, op.as_bytes(), 5000, 1000).unwrap();
    }
    let barrier = Arc::new(Barrier::new(2));
    let handles: Vec<_> = ["one", "two"]
        .into_iter()
        .map(|op| {
            let root = temp.path().join("private");
            let barrier = barrier.clone();
            thread::spawn(move || {
                let store = EvidenceStore::open(&root, StoreLimits::default()).unwrap();
                barrier.wait();
                store.compare_and_swap_head("task", None, op, 1001).unwrap()
            })
        })
        .collect();
    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    assert_eq!(
        results
            .iter()
            .filter(|r| matches!(r, HeadUpdate::Applied(_)))
            .count(),
        1
    );
    assert_eq!(
        results
            .iter()
            .filter(|r| matches!(r, HeadUpdate::Conflict(_)))
            .count(),
        1
    );
    let before = store.read_head("task", 1002).unwrap().unwrap();
    assert!(matches!(
        store
            .compare_and_swap_head("task", None, "three", 1003)
            .unwrap(),
        HeadUpdate::Conflict(_)
    ));
    assert_eq!(
        store.read_head("task", 1003).unwrap().unwrap().identity,
        before.identity
    );
    assert!(matches!(
        store
            .compare_and_swap_head("task", Some(&before.identity), "three", 1003)
            .unwrap(),
        HeadUpdate::Applied(_)
    ));
}

#[test]
fn aba_wrong_key_and_content_tokens_cannot_replace_current_head() {
    let (_temp, store) = fixture();
    for op in ["a", "b", "c"] {
        store.put(op, op.as_bytes(), 5000, 1000).unwrap();
    }
    let HeadUpdate::Applied(first) = store
        .compare_and_swap_head("task", None, "a", 1001)
        .unwrap()
    else {
        panic!()
    };
    let HeadUpdate::Applied(second) = store
        .compare_and_swap_head("task", Some(&first), "b", 1002)
        .unwrap()
    else {
        panic!()
    };
    let HeadUpdate::Applied(third) = store
        .compare_and_swap_head("task", Some(&second), "a", 1003)
        .unwrap()
    else {
        panic!()
    };
    assert!(third.revision > first.revision);
    assert!(matches!(
        store
            .compare_and_swap_head("task", Some(&first), "c", 1004)
            .unwrap(),
        HeadUpdate::Conflict(_)
    ));
    let mut forged = third.clone();
    forged.content_digest = second.content_digest.clone();
    assert!(matches!(
        store
            .compare_and_swap_head("task", Some(&forged), "c", 1004)
            .unwrap(),
        HeadUpdate::Conflict(_)
    ));
    assert!(matches!(
        store
            .compare_and_swap_head("other-task", Some(&third), "c", 1004)
            .unwrap(),
        HeadUpdate::Conflict(None)
    ));
    assert_eq!(
        store.read_head("task", 1004).unwrap().unwrap().identity,
        third
    );
}

#[test]
fn pending_missing_corrupt_quarantined_and_expired_cannot_become_heads() {
    let (_temp, store) = fixture();
    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = store.put_observed("pending", b"pending", 5000, 1000, |phase| {
            if phase == guard_store::WritePhase::IntentDurable {
                panic!("injected interruption");
            }
        });
    }));
    assert!(panic.is_err());
    assert!(
        store
            .compare_and_swap_head("task", None, "pending", 1001)
            .is_err()
    );
    assert!(
        store
            .compare_and_swap_head("task", None, "never-written", 1001)
            .is_err()
    );
    for op in ["missing", "corrupt", "quarantined", "expired"] {
        store.put(op, b"data", 1100, 1000).unwrap();
    }
    std::fs::remove_file(store.artifact_path("missing").unwrap()).unwrap();
    std::fs::write(store.artifact_path("corrupt").unwrap(), b"bad!").unwrap();
    std::fs::remove_file(store.artifact_path("quarantined").unwrap()).unwrap();
    assert!(store.read_private("quarantined", 1001).is_err());
    for op in ["missing", "corrupt", "quarantined"] {
        assert!(store.compare_and_swap_head("task", None, op, 1002).is_err());
    }
    assert!(
        store
            .compare_and_swap_head("task", None, "expired", 1100)
            .is_err()
    );
    assert!(store.read_head("task", 1002).unwrap().is_none());
    assert!(store.head_audit_events().unwrap().is_empty());
}

#[test]
fn existing_head_is_reverified_even_for_idempotent_cas() {
    for failure in ["missing", "corrupt", "quarantined", "expired"] {
        let (_temp, store) = fixture();
        store.put("old", b"data", 1100, 1000).unwrap();
        store.put("new", b"next", 5000, 1000).unwrap();
        let HeadUpdate::Applied(identity) = store
            .compare_and_swap_head("task", None, "old", 1001)
            .unwrap()
        else {
            panic!()
        };
        match failure {
            "missing" => std::fs::remove_file(store.artifact_path("old").unwrap()).unwrap(),
            "corrupt" => std::fs::write(store.artifact_path("old").unwrap(), b"bad!").unwrap(),
            "quarantined" => {
                std::fs::remove_file(store.artifact_path("old").unwrap()).unwrap();
                assert!(store.read_private("old", 1002).is_err());
            }
            "expired" => {
                store.expire(1100).unwrap();
            }
            _ => unreachable!(),
        }
        let now = if failure == "expired" { 1101 } else { 1003 };
        assert!(store.read_head("task", now).is_err());
        assert!(
            store
                .compare_and_swap_head("task", Some(&identity), "new", now)
                .is_err()
        );
        assert!(
            store
                .compare_and_swap_head("task", None, "old", now)
                .is_err()
        );
        assert_eq!(store.head_audit_events().unwrap().len(), 1);
    }
}

#[test]
fn head_and_history_quotas_fail_without_deleting_or_changing_history() {
    let temp = tempfile::tempdir().unwrap();
    let limits = StoreLimits {
        max_records: 1,
        ..StoreLimits::default()
    };
    let store = EvidenceStore::create(&temp.path().join("private"), limits).unwrap();
    store.put("one", b"one", 5000, 1000).unwrap();
    let HeadUpdate::Applied(identity) = store
        .compare_and_swap_head("task", None, "one", 1001)
        .unwrap()
    else {
        panic!()
    };
    assert!(
        store
            .compare_and_swap_head("other", None, "one", 1001)
            .is_err()
    );
    let mut wrong = identity.clone();
    wrong.revision += 10;
    for _ in 0..3 {
        assert!(matches!(
            store
                .compare_and_swap_head("task", Some(&wrong), "one", 1002)
                .unwrap(),
            HeadUpdate::Conflict(_)
        ));
    }
    assert_eq!(store.head_audit_events().unwrap().len(), 4);
    assert!(
        store
            .compare_and_swap_head("task", Some(&wrong), "one", 1002)
            .is_err()
    );
    assert_eq!(
        store.read_head("task", 1002).unwrap().unwrap().identity,
        identity
    );
    assert!(matches!(
        store
            .compare_and_swap_head("task", None, "one", 1002)
            .unwrap(),
        HeadUpdate::Unchanged(_)
    ));
}

#[test]
fn legacy_schema_is_rejected_without_reinitializing_records() {
    let (temp, store) = fixture();
    store.put("one", b"preserve", 5000, 1000).unwrap();
    drop(store);
    let db = rusqlite::Connection::open(temp.path().join("private/evidence.sqlite3")).unwrap();
    db.execute_batch("PRAGMA user_version=1").unwrap();
    drop(db);
    assert!(EvidenceStore::open(&temp.path().join("private"), StoreLimits::default()).is_err());
    let db = rusqlite::Connection::open(temp.path().join("private/evidence.sqlite3")).unwrap();
    assert_eq!(
        db.query_row("SELECT COUNT(*) FROM records", [], |r| r.get::<_, u64>(0))
            .unwrap(),
        1
    );
    assert_eq!(
        db.query_row("PRAGMA user_version", [], |r| r.get::<_, u64>(0))
            .unwrap(),
        1
    );
}

#[test]
fn competing_processes_have_one_durable_cas_winner() {
    use std::{
        fs,
        process::{Command, Stdio},
        time::{Duration, Instant},
    };
    let (temp, store) = fixture();
    for op in ["left", "right"] {
        store.put(op, op.as_bytes(), 5000, 1000).unwrap();
    }
    let mut children = Vec::new();
    for op in ["left", "right"] {
        children.push(
            Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "head_process_worker", "--nocapture"])
                .env("GUARD_HEAD_TEST_ROOT", temp.path())
                .env("GUARD_HEAD_TEST_OP", op)
                .stdout(Stdio::null())
                .spawn()
                .unwrap(),
        );
    }
    let started = Instant::now();
    while !temp.path().join("left.ready").exists() || !temp.path().join("right.ready").exists() {
        assert!(
            started.elapsed() < Duration::from_secs(20),
            "workers did not become ready"
        );
        thread::sleep(Duration::from_millis(10));
    }
    fs::write(temp.path().join("go"), b"start").unwrap();
    let mut codes: Vec<_> = children
        .iter_mut()
        .map(|c| c.wait().unwrap().code().unwrap())
        .collect();
    codes.sort();
    assert_eq!(codes, vec![81, 82]);
    drop(store);
    let reopened =
        EvidenceStore::open(&temp.path().join("private"), StoreLimits::default()).unwrap();
    let head = reopened.read_head("task", 1002).unwrap().unwrap();
    assert!([b"left".to_vec(), b"right".to_vec()].contains(&head.bytes));
    let events = reopened.head_audit_events().unwrap();
    assert_eq!(events.iter().filter(|e| e.code == "applied").count(), 1);
    assert_eq!(events.iter().filter(|e| e.code == "conflict").count(), 1);
}

#[test]
fn head_process_worker() {
    let Some(root) = std::env::var_os("GUARD_HEAD_TEST_ROOT") else {
        return;
    };
    let op = std::env::var("GUARD_HEAD_TEST_OP").unwrap();
    let root = std::path::PathBuf::from(root);
    let store = EvidenceStore::open(&root.join("private"), StoreLimits::default()).unwrap();
    std::fs::write(root.join(format!("{op}.ready")), b"ready").unwrap();
    let started = std::time::Instant::now();
    while !root.join("go").exists() {
        assert!(started.elapsed() < std::time::Duration::from_secs(20));
        thread::sleep(std::time::Duration::from_millis(10));
    }
    let outcome = store
        .compare_and_swap_head("task", None, &op, 1001)
        .unwrap();
    std::process::exit(match outcome {
        HeadUpdate::Applied(_) => 81,
        HeadUpdate::Conflict(_) => 82,
        _ => 83,
    });
}

#[test]
fn observed_corruption_remains_quarantined_after_original_bytes_are_restored() {
    for entry in ["read", "cas-current", "cas-new", "cas-retry"] {
        let (_temp, store) = fixture();
        store.put("old", b"data", 5000, 1000).unwrap();
        store.put("next", b"next", 5000, 1000).unwrap();
        let identity = if entry == "cas-new" {
            None
        } else {
            let HeadUpdate::Applied(identity) = store
                .compare_and_swap_head("task", None, "old", 1001)
                .unwrap()
            else {
                panic!()
            };
            Some(identity)
        };
        let artifact = store.artifact_path("old").unwrap();
        std::fs::write(&artifact, b"bad!").unwrap();
        match entry {
            "read" => assert!(store.read_head("task", 1002).is_err()),
            "cas-current" => assert!(
                store
                    .compare_and_swap_head("task", identity.as_ref(), "next", 1002)
                    .is_err()
            ),
            "cas-new" => assert!(
                store
                    .compare_and_swap_head("task", None, "old", 1002)
                    .is_err()
            ),
            "cas-retry" => assert!(
                store
                    .compare_and_swap_head("task", None, "old", 1002)
                    .is_err()
            ),
            _ => unreachable!(),
        }
        assert_eq!(
            store.state("old").unwrap(),
            guard_store::RecordState::Quarantined,
            "{entry}"
        );
        std::fs::write(&artifact, b"data").unwrap();
        if identity.is_some() {
            assert!(store.read_head("task", 1003).is_err());
        }
        assert!(
            store
                .compare_and_swap_head("task", None, "old", 1003)
                .is_err()
        );
        assert_eq!(
            store
                .audit_events()
                .unwrap()
                .iter()
                .filter(|e| e.code == "artifact_invalid")
                .count(),
            1
        );
    }
}
