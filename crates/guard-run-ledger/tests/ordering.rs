use guard_protocol::{Digest, Identifier};
use guard_run_ledger::{FinalizeOutcome, RunIdentity, RunLedger, RunScope, RunState, StartOutcome};
use guard_store::{EvidenceStore, StoreLimits};
use std::{
    path::Path,
    process::Command,
    time::{Duration, Instant},
};
fn d(s: &str) -> Digest {
    guard_protocol::digest_json(&serde_json::to_vec(s).unwrap()).unwrap()
}
fn scope() -> RunScope {
    RunScope {
        repository_id: Identifier::new("repo").unwrap(),
        task_id: Identifier::new("task").unwrap(),
        obligation_id: Identifier::new("check").unwrap(),
        approved_scope_digest: d("scope"),
    }
}
fn identity(n: &str) -> RunIdentity {
    RunIdentity {
        binding_digest: d("binding"),
        subject_digest: d(n),
        contract_digest: d("contract"),
        policy_digest: d("policy"),
        verifier_digest: d("verifier"),
        environment_digest: d("environment"),
    }
}
fn store(root: &Path) -> EvidenceStore {
    EvidenceStore::open(root, StoreLimits::default()).unwrap()
}
#[test]
fn late_pass_after_new_failure_is_audit_only_and_old_input_is_stale() {
    let tmp = tempfile::tempdir().unwrap();
    let store = EvidenceStore::create(&tmp.path().join("store"), StoreLimits::default()).unwrap();
    let ledger = RunLedger::new(&store);
    let a = ledger
        .start(&scope(), &identity("a"), "one", 9000, 1000)
        .unwrap()
        .into_ticket();
    assert_eq!(
        ledger.finalize(&a, RunState::Fail, b"first", 1001).unwrap(),
        FinalizeOutcome::Applied
    );
    let b = ledger
        .start(&scope(), &identity("b"), "two", 9000, 1002)
        .unwrap()
        .into_ticket();
    assert_eq!(b.sequence().as_str(), "2");
    assert_eq!(
        ledger
            .finalize(&b, RunState::Fail, b"new fail", 1003)
            .unwrap(),
        FinalizeOutcome::Applied
    );
    assert_eq!(
        ledger
            .finalize(&a, RunState::Pass, b"late pass", 1004)
            .unwrap(),
        FinalizeOutcome::AuditOnly
    );
    assert_eq!(
        ledger
            .inspect(&scope(), &identity("b"), 1005)
            .unwrap()
            .unwrap()
            .state,
        RunState::Fail
    );
    assert_eq!(
        ledger
            .inspect(&scope(), &identity("a"), 1005)
            .unwrap()
            .unwrap()
            .state,
        RunState::Stale
    );
}
#[test]
fn private_binary_output_up_to_declared_bound_is_not_a_json_array() {
    let tmp = tempfile::tempdir().unwrap();
    let store = EvidenceStore::create(&tmp.path().join("store"), StoreLimits::default()).unwrap();
    let ledger = RunLedger::new(&store);
    for (index, size) in [16384, 4 * 1024 * 1024].into_iter().enumerate() {
        let ticket = ledger
            .start(
                &scope(),
                &identity("large"),
                &format!("large-{index}"),
                9000,
                1000,
            )
            .unwrap()
            .into_ticket();
        let raw = vec![0xff; size];
        assert_eq!(
            ledger
                .finalize(&ticket, RunState::Incomplete, &raw, 1001)
                .unwrap(),
            FinalizeOutcome::Applied
        );
        assert!(
            ledger
                .inspect(&scope(), &identity("large"), 1002)
                .unwrap()
                .unwrap()
                .result_digest
                .is_some()
        );
    }
}
#[test]
fn real_process_crash_recovers_incomplete_and_cannot_resume_pass() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("store");
    EvidenceStore::create(&root, StoreLimits::default()).unwrap();
    let child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "worker", "--nocapture"])
        .env("LEDGER_ROOT", &root)
        .env("LEDGER_CRASH", "1")
        .status()
        .unwrap();
    assert_eq!(child.code(), Some(81));
    let store = store(&root);
    let ledger = RunLedger::new(&store);
    assert!(ledger.recover(&scope(), 1002).unwrap());
    assert_eq!(
        ledger
            .inspect(&scope(), &identity("a"), 1003)
            .unwrap()
            .unwrap()
            .state,
        RunState::Incomplete
    );
    let next = ledger
        .start(&scope(), &identity("a"), "next", 9000, 1004)
        .unwrap()
        .into_ticket();
    assert_eq!(next.sequence().as_str(), "2");
}
#[test]
fn two_real_processes_only_one_start_is_dispatchable() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("store");
    EvidenceStore::create(&root, StoreLimits::default()).unwrap();
    let barrier = tmp.path().join("barrier");
    std::fs::create_dir(&barrier).unwrap();
    let mut a = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "worker", "--nocapture"])
        .env("LEDGER_ROOT", &root)
        .env("LEDGER_BARRIER", &barrier)
        .spawn()
        .unwrap();
    let mut b = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "worker", "--nocapture"])
        .env("LEDGER_ROOT", &root)
        .env("LEDGER_BARRIER", &barrier)
        .spawn()
        .unwrap();
    // 两个worker均完成只读打开后再竞争start，不把准备阶段300ms忙碌冒认为CAS结果。
    let deadline = Instant::now() + Duration::from_secs(10);
    while std::fs::read_dir(&barrier).unwrap().count() != 2 {
        assert!(
            Instant::now() < deadline,
            "workers did not finish bounded store preparation"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    std::fs::write(barrier.join("go"), b"start").unwrap();
    let mut codes = vec![
        a.wait().unwrap().code().unwrap(),
        b.wait().unwrap().code().unwrap(),
    ];
    codes.sort();
    assert_eq!(codes, vec![81, 82]);
    let opened = store(&root);
    let active = RunLedger::new(&opened)
        .inspect(&scope(), &identity("a"), 1001)
        .unwrap()
        .unwrap();
    assert_eq!(active.state, RunState::Running);
    assert_eq!(active.sequence.as_str(), "1");
}
#[test]
fn worker() {
    let Some(root) = std::env::var_os("LEDGER_ROOT") else {
        return;
    };
    // CI慢盘可在恢复/打开阶段超过默认busy上限；测试宿主仅对明确busy有限重试。
    // 生产StoreLimits及start本身的等待/失败语义保持不变，其他错误仍直接失败。
    let deadline = Instant::now() + Duration::from_secs(5);
    let store = loop {
        match EvidenceStore::open(Path::new(&root), StoreLimits::default()) {
            Ok(store) => break store,
            Err(error) if error == "store busy" && Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(error) => panic!("worker store preparation failed: {error}"),
        }
    };
    if let Some(barrier) = std::env::var_os("LEDGER_BARRIER") {
        let barrier = Path::new(&barrier);
        std::fs::write(
            barrier.join(format!("{}.ready", std::process::id())),
            b"ready",
        )
        .unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        while !barrier.join("go").exists() {
            assert!(Instant::now() < deadline, "start barrier deadline exceeded");
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    let ledger = RunLedger::new(&store);
    let outcome = ledger.start(&scope(), &identity("a"), "worker", 9000, 1000);
    let code = match outcome {
        Ok(StartOutcome::Started(_)) => 81,
        Ok(StartOutcome::Existing(_)) => 82,
        Err(error)
            if error == "store busy"
                || error == "concurrent start lost; no process may be dispatched" =>
        {
            82
        }
        Err(error) => panic!("unexpected start rejection: {error}"),
    };
    std::process::exit(code);
}
#[test]
fn recovery_blocks_old_ticket_pass_and_new_run_keeps_monotonic_sequence_after_restart() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("store");
    let db = EvidenceStore::create(&root, StoreLimits::default()).unwrap();
    let l = RunLedger::new(&db);
    let old = l
        .start(&scope(), &identity("old"), "old", 9000, 1000)
        .unwrap()
        .into_ticket();
    assert!(l.recover(&scope(), 1001).unwrap());
    let current = l
        .start(&scope(), &identity("new"), "new", 9000, 1002)
        .unwrap()
        .into_ticket();
    assert_eq!(current.sequence().as_str(), "2");
    assert_eq!(
        l.finalize(&current, RunState::Fail, b"newest fail", 1003)
            .unwrap(),
        FinalizeOutcome::Applied
    );
    assert_eq!(
        l.finalize(&old, RunState::Pass, b"orphaned old process pass", 1004)
            .unwrap(),
        FinalizeOutcome::AuditOnly
    );
    drop(db);
    let db = store(&root);
    let l = RunLedger::new(&db);
    assert_eq!(
        l.inspect(&scope(), &identity("new"), 1005)
            .unwrap()
            .unwrap()
            .state,
        RunState::Fail
    );
    let next = l
        .start(&scope(), &identity("new"), "third", 9000, 1006)
        .unwrap()
        .into_ticket();
    assert_eq!(next.sequence().as_str(), "3");
}
#[test]
fn all_identity_dimensions_are_checked_without_moving_the_stable_head() {
    let tmp = tempfile::tempdir().unwrap();
    let db = EvidenceStore::create(&tmp.path().join("store"), StoreLimits::default()).unwrap();
    let l = RunLedger::new(&db);
    let original = identity("a");
    let ticket = l
        .start(&scope(), &original, "id", 9000, 1000)
        .unwrap()
        .into_ticket();
    l.finalize(&ticket, RunState::Pass, b"pass", 1001).unwrap();
    for i in 0..6 {
        let mut changed = original.clone();
        let new = d("changed");
        match i {
            0 => changed.binding_digest = new,
            1 => changed.subject_digest = new,
            2 => changed.contract_digest = new,
            3 => changed.policy_digest = new,
            4 => changed.verifier_digest = new,
            _ => changed.environment_digest = new,
        };
        assert_eq!(
            l.inspect(&scope(), &changed, 1002).unwrap().unwrap().state,
            RunState::Stale
        );
    }
    let mut changed_scope = scope();
    changed_scope.approved_scope_digest = d("another authorization range");
    assert!(
        l.start(&changed_scope, &original, "new", 9000, 1002)
            .is_err()
    );
    assert!(l.inspect(&changed_scope, &original, 1002).is_err());
    assert_eq!(
        l.inspect(&scope(), &original, 1002).unwrap().unwrap().state,
        RunState::Pass
    );
}
#[test]
fn duplicate_start_and_finalize_do_not_dispatch_or_rewrite_terminal_result() {
    let tmp = tempfile::tempdir().unwrap();
    let db = EvidenceStore::create(&tmp.path().join("store"), StoreLimits::default()).unwrap();
    let l = RunLedger::new(&db);
    let a = l
        .start(&scope(), &identity("a"), "same", 9000, 1000)
        .unwrap()
        .into_ticket();
    assert!(matches!(
        l.start(&scope(), &identity("a"), "same", 9000, 1001)
            .unwrap(),
        StartOutcome::Existing(_)
    ));
    assert!(
        l.start(&scope(), &identity("b"), "other", 9000, 1001)
            .is_err()
    );
    assert!(l.finalize(&a, RunState::Running, b"", 1002).is_err());
    assert_eq!(
        l.finalize(&a, RunState::Cancelled, b"cancelled", 1002)
            .unwrap(),
        FinalizeOutcome::Applied
    );
    assert_eq!(
        l.finalize(&a, RunState::Cancelled, b"cancelled", 1003)
            .unwrap(),
        FinalizeOutcome::Unchanged
    );
    assert_eq!(
        l.finalize(&a, RunState::Pass, b"different", 1004).unwrap(),
        FinalizeOutcome::AuditOnly
    );
    assert!(
        l.start(&scope(), &identity("a"), "same", 9000, 1005)
            .is_err()
    );
    assert_eq!(
        l.inspect(&scope(), &identity("a"), 1005)
            .unwrap()
            .unwrap()
            .trust,
        "local_unverified"
    );
}
#[test]
fn expired_or_corrupt_private_head_fails_closed_without_restarting_sequence() {
    let tmp = tempfile::tempdir().unwrap();
    let db = EvidenceStore::create(&tmp.path().join("store"), StoreLimits::default()).unwrap();
    let l = RunLedger::new(&db);
    l.start(&scope(), &identity("a"), "one", 1100, 1000)
        .unwrap();
    assert!(l.inspect(&scope(), &identity("a"), 1100).is_err());
    assert!(
        l.start(&scope(), &identity("a"), "two", 9000, 1101)
            .is_err()
    );
}
#[test]
fn corrupt_head_is_quarantined_and_restoring_original_bytes_cannot_restart_it() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("store");
    let db = EvidenceStore::create(&root, StoreLimits::default()).unwrap();
    let ledger = RunLedger::new(&db);
    ledger
        .start(&scope(), &identity("a"), "one", 9000, 1000)
        .unwrap();
    let mut saved = Vec::new();
    for entry in std::fs::read_dir(root.join("artifacts")).unwrap() {
        let path = entry.unwrap().path();
        let bytes = std::fs::read(&path).unwrap();
        if bytes
            .windows(b"guard.run-record/v1".len())
            .any(|w| w == b"guard.run-record/v1")
        {
            std::fs::write(&path, b"corrupt").unwrap();
            saved.push((path, bytes));
        }
    }
    assert!(!saved.is_empty());
    assert!(ledger.inspect(&scope(), &identity("a"), 1001).is_err());
    for (path, bytes) in saved {
        std::fs::write(path, bytes).unwrap();
    }
    assert!(ledger.inspect(&scope(), &identity("a"), 1002).is_err());
    assert!(ledger.recover(&scope(), 1002).is_err());
    assert!(
        ledger
            .start(&scope(), &identity("a"), "new", 9000, 1003)
            .is_err()
    );
}
