#![cfg(feature = "test-fixture")]
use guard_protocol::{Digest, ExecutionBudget, ExecutionStatus};
use guard_runner::{Cancellation, CommandSpec, ProcessRunner, StopReason};
use sha2::{Digest as _, Sha256};
use std::{collections::BTreeMap, path::PathBuf, time::Duration};

fn command(temp: &tempfile::TempDir, args: &[&str]) -> CommandSpec {
    let executable = PathBuf::from(env!("CARGO_BIN_EXE_guard-runner-fixture"));
    let hash = format!(
        "sha256:{:x}",
        Sha256::digest(std::fs::read(&executable).unwrap())
    );
    CommandSpec {
        executable,
        executable_digest: Digest::new(hash).unwrap(),
        args: args.iter().map(|v| v.to_string()).collect(),
        directory: temp.path().to_path_buf(),
        environment: BTreeMap::new(),
    }
}
fn budget(timeout_ms: u64, max_output_bytes: u64) -> ExecutionBudget {
    ExecutionBudget {
        timeout_ms,
        max_output_bytes,
        max_concurrency: 2,
    }
}

#[tokio::test]
async fn successful_exit_is_only_an_execution_observation_and_preserves_both_streams() {
    let dir = tempfile::tempdir().unwrap();
    let runner = ProcessRunner::new(budget(5000, 4096)).unwrap();
    let result = runner
        .run(command(&dir, &["output"]), Cancellation::new())
        .await
        .unwrap();
    assert_eq!(result.status, ExecutionStatus::Completed);
    assert_eq!(result.exit_code, Some(0));
    assert_eq!(result.reason, StopReason::Exited);
    let started_at = result
        .started_at
        .as_ref()
        .expect("actual process start observation");
    let finished_at = result
        .finished_at
        .as_ref()
        .expect("actual process completion observation");
    assert!(!started_at.compare_instant(finished_at).is_gt());
    assert!(String::from_utf8_lossy(&result.stdout).contains("confirmed finding"));
    assert!(String::from_utf8_lossy(&result.stderr).contains("private diagnostic"));
}

#[tokio::test]
async fn timeout_and_cancellation_keep_partial_findings() {
    let dir = tempfile::tempdir().unwrap();
    let runner = ProcessRunner::new(budget(5000, 4096)).unwrap();
    let result = runner
        .run(command(&dir, &["sleep"]), Cancellation::new())
        .await
        .unwrap();
    assert_eq!(result.status, ExecutionStatus::TimedOut);
    assert!(String::from_utf8_lossy(&result.stdout).contains("confirmed finding"));
    let cancel = Cancellation::new();
    let trigger = cancel.clone();
    let marker = dir.path().join("flushed-before-cancel");
    let ready = marker.clone();
    tokio::spawn(async move {
        for _ in 0..200 {
            if ready.exists() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        trigger.cancel();
    });
    let result = runner
        .run(command(&dir, &["sleep", marker.to_str().unwrap()]), cancel)
        .await
        .unwrap();
    assert!(
        marker.exists(),
        "fixture must produce actual partial output before cancellation"
    );
    assert_eq!(result.status, ExecutionStatus::Cancelled);
    assert!(String::from_utf8_lossy(&result.stdout).contains("confirmed finding"));
}

#[tokio::test]
async fn output_budget_is_aggregate_and_never_returns_complete_success() {
    let dir = tempfile::tempdir().unwrap();
    let runner = ProcessRunner::new(budget(5000, 2000)).unwrap();
    let result = runner
        .run(command(&dir, &["flood"]), Cancellation::new())
        .await
        .unwrap();
    assert_eq!(result.reason, StopReason::OutputLimit);
    assert_eq!(result.status, ExecutionStatus::Failed);
    assert_eq!(result.stdout.len() + result.stderr.len(), 2000);
    assert!(result.output_truncated);
}

#[tokio::test]
async fn argv_is_literal_environment_is_explicit_and_wrong_executable_pin_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let runner = ProcessRunner::new(budget(5000, 4096)).unwrap();
    let result = runner
        .run(
            command(&dir, &["args", "a b", "$(not-a-command)", "a\"b", "tail\\"]),
            Cancellation::new(),
        )
        .await
        .unwrap();
    assert_eq!(
        String::from_utf8(result.stdout).unwrap().trim(),
        r#"["a b", "$(not-a-command)", "a\"b", "tail\\"]"#
    );
    let mut spec = command(&dir, &["environment"]);
    spec.environment
        .insert("GUARD_EXPLICIT".into(), "visible".into());
    let result = runner.run(spec, Cancellation::new()).await.unwrap();
    assert!(!std::env::var("PATH").unwrap().is_empty());
    assert_eq!(
        String::from_utf8(result.stdout).unwrap().replace('\r', ""),
        "\n\nvisible\n"
    );
    let mut wrong = command(&dir, &["output"]);
    wrong.executable_digest = Digest::new(format!("sha256:{}", "0".repeat(64))).unwrap();
    assert!(runner.run(wrong, Cancellation::new()).await.is_err());
}

#[tokio::test]
async fn cancellation_and_normal_parent_exit_stop_descendants() {
    let dir = tempfile::tempdir().unwrap();
    for normal_exit in [false, true] {
        let heartbeat = dir
            .path()
            .join(if normal_exit { "normal" } else { "cancelled" });
        let path = heartbeat.to_str().unwrap();
        let args = if normal_exit {
            vec!["tree", path, "exit"]
        } else {
            vec!["tree", path]
        };
        let runner = ProcessRunner::new(budget(5000, 4096)).unwrap();
        let cancel = Cancellation::new();
        let trigger = cancel.clone();
        if !normal_exit {
            let heartbeat = heartbeat.clone();
            tokio::spawn(async move {
                for _ in 0..100 {
                    if heartbeat.exists() {
                        break;
                    }
                    tokio::time::sleep(Duration::from_millis(20)).await;
                }
                trigger.cancel();
            });
        }
        let result = runner.run(command(&dir, &args), cancel).await.unwrap();
        assert_eq!(
            result.status,
            if normal_exit {
                ExecutionStatus::Completed
            } else {
                ExecutionStatus::Cancelled
            }
        );
        // 完成后缓冲活动停止；不是仅凭父进程退出码声称子孙已停止。
        tokio::time::sleep(Duration::from_millis(150)).await;
        let before = std::fs::read(&heartbeat).ok();
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert_eq!(std::fs::read(&heartbeat).ok(), before);
        if !normal_exit {
            assert!(before.is_some());
        }
    }
}

#[tokio::test]
async fn failed_exit_and_pre_cancel_do_not_run_as_success() {
    let dir = tempfile::tempdir().unwrap();
    let runner = ProcessRunner::new(budget(5000, 4096)).unwrap();
    let result = runner
        .run(command(&dir, &["fail"]), Cancellation::new())
        .await
        .unwrap();
    assert_eq!(result.status, ExecutionStatus::Failed);
    assert_eq!(result.exit_code, Some(7));
    let cancel = Cancellation::new();
    cancel.cancel();
    let result = runner
        .run(command(&dir, &["output"]), cancel)
        .await
        .unwrap();
    assert_eq!(result.status, ExecutionStatus::Cancelled);
    assert!(result.stdout.is_empty());
    assert!(result.process_id.is_none());
    assert!(result.started_at.is_none());
    assert!(result.finished_at.is_none());
}

#[tokio::test]
async fn shared_concurrency_budget_queues_without_executing_and_cancelled_queue_has_no_pid() {
    let dir = tempfile::tempdir().unwrap();
    let heartbeat = dir.path().join("busy");
    let marker = dir.path().join("queued");
    let mut limits = budget(10000, 4096);
    limits.max_concurrency = 1;
    let runner = ProcessRunner::new(limits).unwrap();
    let first = command(&dir, &["tree", heartbeat.to_str().unwrap()]);
    let cancel = Cancellation::new();
    let trigger = cancel.clone();
    let first_runner = runner.clone();
    let running = tokio::spawn(async move { first_runner.run(first, cancel).await });
    for _ in 0..200 {
        if heartbeat.exists() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(heartbeat.exists());
    let queued_cancel = Cancellation::new();
    let cancel_queued = queued_cancel.clone();
    let queued_runner = runner.clone();
    let second = command(&dir, &["touch", marker.to_str().unwrap()]);
    let queued = tokio::spawn(async move { queued_runner.run(second, queued_cancel).await });
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(!marker.exists());
    cancel_queued.cancel();
    let stopped = queued.await.unwrap().unwrap();
    assert_eq!(stopped.status, ExecutionStatus::Cancelled);
    assert!(stopped.process_id.is_none());
    trigger.cancel();
    assert_eq!(
        running.await.unwrap().unwrap().status,
        ExecutionStatus::Cancelled
    );
    let result = runner
        .run(
            command(&dir, &["touch", marker.to_str().unwrap()]),
            Cancellation::new(),
        )
        .await
        .unwrap();
    assert_eq!(result.status, ExecutionStatus::Completed);
    assert!(marker.exists());
}

#[tokio::test]
async fn dropping_execution_future_terminates_its_descendant_tree() {
    let dir = tempfile::tempdir().unwrap();
    let heartbeat = dir.path().join("abort");
    let spec = command(&dir, &["tree", heartbeat.to_str().unwrap()]);
    let runner = ProcessRunner::new(budget(10000, 4096)).unwrap();
    let running = tokio::spawn(async move { runner.run(spec, Cancellation::new()).await });
    for _ in 0..200 {
        if heartbeat.exists() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(heartbeat.exists());
    running.abort();
    assert!(running.await.unwrap_err().is_cancelled());
    tokio::time::sleep(Duration::from_millis(150)).await;
    let before = std::fs::read(&heartbeat).unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(std::fs::read(&heartbeat).unwrap(), before);
}

#[cfg(windows)]
#[test]
fn abrupt_controller_death_closes_windows_job_and_stops_descendants() {
    let dir = tempfile::tempdir().unwrap();
    let heartbeat = dir.path().join("host-crash");
    let mut worker = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "crash_worker", "--nocapture"])
        .env("GUARD_RUNNER_CRASH_DIRECTORY", dir.path())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    for _ in 0..200 {
        if heartbeat.exists() {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    if !heartbeat.exists() {
        let _ = worker.kill();
        let _ = worker.wait();
        panic!("actual child tree did not start before controller crash");
    }
    worker.kill().unwrap();
    worker.wait().unwrap();
    std::thread::sleep(Duration::from_millis(150));
    let before = std::fs::read(&heartbeat).unwrap();
    std::thread::sleep(Duration::from_millis(200));
    assert_eq!(std::fs::read(&heartbeat).unwrap(), before);
}

#[cfg(windows)]
#[test]
fn crash_worker() {
    let Some(directory) = std::env::var_os("GUARD_RUNNER_CRASH_DIRECTORY") else {
        return;
    };
    let heartbeat = PathBuf::from(directory).join("host-crash");
    let dir = tempfile::tempdir().unwrap();
    let spec = command(&dir, &["tree", heartbeat.to_str().unwrap()]);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        ProcessRunner::new(budget(30000, 4096))
            .unwrap()
            .run(spec, Cancellation::new())
            .await
            .unwrap();
    });
    panic!("crash worker must be terminated by the external test process");
}

#[cfg(windows)]
#[tokio::test]
async fn every_managed_descendant_handle_is_signaled_when_run_returns() {
    use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
    use windows_sys::Win32::{
        Foundation::{WAIT_OBJECT_0, WAIT_TIMEOUT},
        System::Threading::{OpenProcess, PROCESS_SYNCHRONIZE, WaitForSingleObject},
    };
    for stop in ["exit", "cancel", "timeout", "flood"] {
        let dir = tempfile::tempdir().unwrap();
        let runner = ProcessRunner::new(budget(10000, 4096)).unwrap();
        let spec = command(&dir, &["forest", dir.path().to_str().unwrap(), stop]);
        let cancel = Cancellation::new();
        let trigger = cancel.clone();
        let running = tokio::spawn(async move { runner.run(spec, cancel).await });
        tokio::time::timeout(Duration::from_secs(8), async {
            while !dir.path().join("forest.pids").exists() {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .expect("actual forest must start before stop trigger");
        let pids = std::fs::read_to_string(dir.path().join("forest.pids")).unwrap();
        let handles: Vec<OwnedHandle> = pids
            .lines()
            .map(|pid| {
                let handle = unsafe { OpenProcess(PROCESS_SYNCHRONIZE, 0, pid.parse().unwrap()) };
                assert!(
                    !handle.is_null(),
                    "retain real live process handle before termination"
                );
                let handle = unsafe { OwnedHandle::from_raw_handle(handle) };
                assert_eq!(
                    unsafe { WaitForSingleObject(handle.as_raw_handle(), 0) },
                    WAIT_TIMEOUT
                );
                handle
            })
            .collect();
        assert_eq!(
            handles.len(),
            5,
            "parent, two children, branch and grandchild"
        );
        if stop == "cancel" {
            trigger.cancel();
        }
        if stop == "exit" || stop == "flood" {
            std::fs::write(dir.path().join("release"), b"go").unwrap();
        }
        let result = running.await.unwrap().unwrap();
        // 第一项返回后观察就是内核句柄状态，不先sleep，也不重新按可复用PID打开句柄。
        for handle in handles {
            assert_eq!(
                unsafe { WaitForSingleObject(handle.as_raw_handle(), 0) },
                WAIT_OBJECT_0,
                "{stop}: a managed process is not terminated at run return"
            );
        }
        assert_eq!(
            result.status,
            match stop {
                "exit" => ExecutionStatus::Completed,
                "cancel" => ExecutionStatus::Cancelled,
                "timeout" => ExecutionStatus::TimedOut,
                _ => ExecutionStatus::Failed,
            }
        );
        assert!(String::from_utf8_lossy(&result.stdout).contains("confirmed finding"));
    }
}

#[tokio::test]
async fn absolute_deadline_can_shorten_a_run_and_preserves_partial_output() {
    let dir = tempfile::tempdir().unwrap();
    let runner = ProcessRunner::new(budget(30000, 4096)).unwrap();
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    let result = runner
        .run_until(command(&dir, &["sleep"]), Cancellation::new(), deadline)
        .await
        .unwrap();
    assert_eq!(result.status, ExecutionStatus::TimedOut);
    assert_eq!(result.reason, StopReason::Timeout);
    assert!(String::from_utf8_lossy(&result.stdout).contains("confirmed finding"));
}

#[tokio::test]
async fn expired_deadline_never_starts_and_a_future_deadline_cannot_extend_own_budget() {
    let dir = tempfile::tempdir().unwrap();
    let marker = dir.path().join("must-not-run");
    let runner = ProcessRunner::new(budget(2000, 4096)).unwrap();
    let expired = std::time::Instant::now() - Duration::from_secs(1);
    let result = runner
        .run_until(
            command(&dir, &["touch", marker.to_str().unwrap()]),
            Cancellation::new(),
            expired,
        )
        .await
        .unwrap();
    assert_eq!(result.status, ExecutionStatus::TimedOut);
    assert!(result.process_id.is_none());
    assert!(!marker.exists());
    let started = std::time::Instant::now();
    let result = runner
        .run_until(
            command(&dir, &["sleep"]),
            Cancellation::new(),
            started + Duration::from_secs(30),
        )
        .await
        .unwrap();
    assert_eq!(result.status, ExecutionStatus::TimedOut);
    assert!(
        started.elapsed() < Duration::from_secs(10),
        "caller deadline must not extend fixed execution budget"
    );
}
