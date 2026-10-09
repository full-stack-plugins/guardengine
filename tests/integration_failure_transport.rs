//! Real Linux process faults around the opt-in publication surface; no current/eligibility claim.
#![cfg(target_os = "linux")]
use guardengine::{Decision, integration::*};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{BufRead, BufReader, Write},
    os::unix::process::ExitStatusExt,
    path::Path,
    process::{Command, Stdio},
    sync::mpsc,
    time::{Duration, Instant},
};

fn base() -> GuardRunEnvelope {
    serde_json::from_slice(include_bytes!(
        "fixtures/integration-envelope/valid-native.json"
    ))
    .unwrap()
}
fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}
fn bound(value: &GuardRunEnvelope) -> BoundAttempt {
    prepare_attempt(InvocationDraft {
        run_id: value.run_id.clone(),
        producer: Some(value.producer.clone()),
        binding: Some(value.binding.clone()),
        coverage: Some(value.coverage.clone()),
        profile: Some(EvidenceProfile::NativeOnly),
        started_at: value.started_at.clone(),
    })
    .unwrap()
}
fn outcome(value: &GuardRunEnvelope) -> AttemptOutput {
    AttemptOutput {
        coverage: value.coverage.clone(),
        run_status: value.run_status.clone(),
        decision: value.decision.clone(),
        artifacts: value.artifacts.clone(),
        approval_refs: vec![],
        diagnostics: value.diagnostics.clone(),
        finished_at: value.finished_at.clone(),
        expires_at: None,
    }
}
fn partial(run_id: &str, diagnostic_bytes: &[u8]) -> GuardRunEnvelope {
    let mut value = base();
    value.run_id = run_id.into();
    value.coverage.status = CoverageStatus::Partial;
    value.coverage.observed_scopes.clear();
    value.coverage.missing_scopes = value.coverage.required_scopes.clone();
    value.decision = Some(Decision::Block);
    value.artifacts.domain = vec![ArtifactRef {
        uri: "artifact://fault-fixture/diagnostic.bin".into(),
        digest: digest(diagnostic_bytes),
        media_type: "application/octet-stream".into(),
    }];
    bound(&value).finish(outcome(&value)).unwrap()
}
fn save_capture(name: &str, files: &[(&str, &[u8])]) {
    if let Some(root) = std::env::var_os("GE_FAULT_CAPTURE_DIR") {
        let root = Path::new(&root).join(name);
        fs::create_dir_all(&root).unwrap();
        for (name, bytes) in files {
            fs::write(root.join(name), bytes).unwrap();
        }
    }
}

// Invoked only by the parent test below; an ordinary suite run returns immediately.
#[test]
fn process_fixture_entry() {
    let Ok(mode) = std::env::var("GE_FAULT_CHILD_MODE") else {
        return;
    };
    let root = std::env::var_os("GE_FAULT_CHILD_ROOT").unwrap();
    let root = Path::new(&root);
    let diagnostic = b"fixture analyzer reached staging; required lint scope remains missing\n";
    fs::write(root.join("diagnostic.bin"), diagnostic).unwrap();
    fs::File::open(root.join("diagnostic.bin"))
        .unwrap()
        .sync_all()
        .unwrap();
    let value = partial(&mode, diagnostic);
    let _staged = stage_attempt(root, &value, EvidenceProfile::NativeOnly).unwrap();
    println!("FAULT_STAGE_READY");
    std::io::stdout().flush().unwrap();
    eprintln!("fixture staged bytes; final publication has not happened");
    match mode.as_str() {
        "crash-after-stage" => std::process::abort(),
        "cancel-after-stage" => loop {
            std::thread::park();
        },
        _ => panic!("unsupported fixture mode"),
    }
}

#[test]
fn actual_crash_and_cancel_preserve_diagnostics_without_publishing_or_overwriting() {
    for mode in ["crash-after-stage", "cancel-after-stage"] {
        let root = tempfile::tempdir().unwrap();
        let old = stage_attempt(root.path(), &base(), EvidenceProfile::NativeOnly)
            .unwrap()
            .publish()
            .unwrap();
        let old_bytes = fs::read(&old.path).unwrap();
        // Limit core dumps in the child; actual SIGABRT is still observed, not a status flag fixture.
        let executable = std::env::current_exe().unwrap();
        let mut child = Command::new("sh")
            .args(["-c", "ulimit -c 0; exec \"$@\"", "ge-fault-fixture"])
            .arg(&executable)
            .args(["--exact", "process_fixture_entry", "--nocapture"])
            .env("GE_FAULT_CHILD_MODE", mode)
            .env("GE_FAULT_CHILD_ROOT", root.path())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let stdout = child.stdout.take().unwrap();
        let (ready_tx, ready_rx) = mpsc::channel();
        let reader = std::thread::spawn(move || {
            let mut output = Vec::new();
            for line in BufReader::new(stdout).split(b'\n') {
                let line = line.unwrap();
                if line
                    .windows(b"FAULT_STAGE_READY".len())
                    .any(|s| s == b"FAULT_STAGE_READY")
                {
                    ready_tx.send(()).unwrap();
                }
                output.extend_from_slice(&line);
                output.push(b'\n');
            }
            output
        });
        if ready_rx.recv_timeout(Duration::from_secs(5)).is_err() {
            let _ = child.kill();
            let _ = child.wait();
            panic!("child never reached staged-file boundary");
        }
        if mode == "cancel-after-stage" {
            child.kill().unwrap();
        }
        let deadline = Instant::now() + Duration::from_secs(5);
        while child.try_wait().unwrap().is_none() {
            if Instant::now() > deadline {
                let _ = child.kill();
                let _ = child.wait();
                panic!("child did not exit");
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let result = child.wait_with_output().unwrap();
        let stdout = reader.join().unwrap();
        assert_eq!(
            result.status.signal(),
            Some(if mode == "crash-after-stage" { 6 } else { 9 })
        );
        let destination = root.path().join(format!(
            "attempt-{:x}.json",
            Sha256::digest(mode.as_bytes())
        ));
        assert!(!destination.exists());
        assert_eq!(fs::read(&old.path).unwrap(), old_bytes);
        let staged_files: Vec<_> = fs::read_dir(root.path())
            .unwrap()
            .map(Result::unwrap)
            .filter(|e| {
                e.file_name()
                    .to_string_lossy()
                    .starts_with(".guard-attempt-")
            })
            .collect();
        assert_eq!(
            staged_files.len(),
            1,
            "crash leaves an orphan stage, not a published attempt"
        );
        let orphan_bytes = fs::read(staged_files[0].path()).unwrap();
        let orphan: GuardRunEnvelope = serde_json::from_slice(&orphan_bytes).unwrap();
        assert_eq!(orphan.decision, Some(Decision::Block));
        let diagnostic = fs::read(root.path().join("diagnostic.bin")).unwrap();
        let mut failure = partial(mode, &diagnostic);
        failure.run_status = if mode == "crash-after-stage" {
            RunStatus::Error
        } else {
            RunStatus::Cancelled
        };
        failure.decision = None;
        failure.diagnostics = vec![Diagnostic {
            code: "fixture.process_terminated".into(),
            message: format!(
                "observed child termination signal {}",
                result.status.signal().unwrap()
            ),
            retryable: false,
            source: Some(failure.artifacts.domain[0].uri.clone()),
        }];
        let mut invalid = outcome(&failure);
        invalid.decision = Some(Decision::Allow);
        assert!(bound(&failure).finish(invalid).is_err());
        let failure = bound(&failure).finish(outcome(&failure)).unwrap();
        let recovered = stage_attempt(root.path(), &failure, EvidenceProfile::NativeOnly)
            .unwrap()
            .publish()
            .unwrap();
        let recovered_bytes = fs::read(&recovered.path).unwrap();
        let verified = load_envelope_json(&recovered_bytes, EvidenceProfile::NativeOnly).unwrap();
        assert_eq!(verified.decision, None);
        assert_eq!(verified.binding, base().binding);
        assert_eq!(verified.run_id, mode);
        assert_eq!(verified.coverage, orphan.coverage);
        assert!(verified.artifacts.report.is_none());
        assert_eq!(verified.artifacts.domain[0].digest, digest(&diagnostic));
        assert_eq!(recovered.content_digest, digest(&recovered_bytes));
        assert_eq!(fs::read(&old.path).unwrap(), old_bytes);
        // Replaying stale completed bytes under this same run ID is an actual no-clobber I/O failure.
        assert!(
            stage_attempt(root.path(), &orphan, EvidenceProfile::NativeOnly)
                .unwrap()
                .publish()
                .is_err()
        );
        assert_eq!(fs::read(&recovered.path).unwrap(), recovered_bytes);
        let metadata = serde_json::to_vec_pretty(&serde_json::json!({
            "fixture": mode, "child_executable": executable, "termination_signal": result.status.signal(),
            "command": ["sh", "-c", "ulimit -c 0; exec \"$@\"", "ge-fault-fixture",
                executable.to_str().unwrap(), "--exact", "process_fixture_entry", "--nocapture"],
            "platform": [std::env::consts::OS, std::env::consts::ARCH],
            "stdout_sha256": digest(&stdout), "stderr_sha256": digest(&result.stderr),
            "recovery_run_status": verified.run_status,
            "fault_injection": if mode == "crash-after-stage" { "child process abort after stage sync" } else { "parent SIGKILL after child stage-ready handshake" },
            "current_eligibility_claimed": false, "authenticated_artifact_claimed": false,
            "diagnostic_sha256": digest(&diagnostic), "recovered_envelope_sha256": digest(&recovered_bytes),
            "old_output_sha256": digest(&old_bytes), "orphan_sha256": digest(&orphan_bytes),
            "old_final_preserved": true, "stale_republish_rejected": true
        })).unwrap();
        save_capture(
            mode,
            &[
                ("diagnostic.bin", &diagnostic),
                ("stdout.raw", &stdout),
                ("stderr.raw", &result.stderr),
                ("old-final.json", &old_bytes),
                ("orphan-stage.json", &orphan_bytes),
                ("recovered-error.json", &recovered_bytes),
                ("capture.json", &metadata),
            ],
        );
    }
}

#[test]
fn completed_partial_is_block_while_failed_outcome_is_null() {
    let value = partial("completed-partial", b"missing required lint scope\n");
    assert_eq!(value.run_status, RunStatus::Completed);
    assert_eq!(value.decision, Some(Decision::Block));
    let mut invalid = outcome(&value);
    invalid.decision = Some(Decision::Allow);
    assert!(bound(&value).finish(invalid).is_err());
    let root = tempfile::tempdir().unwrap();
    let published = stage_attempt(root.path(), &value, EvidenceProfile::NativeOnly)
        .unwrap()
        .publish()
        .unwrap();
    let bytes = fs::read(published.path).unwrap();
    assert_eq!(published.content_digest, digest(&bytes));
    save_capture("completed-partial", &[("envelope.json", &bytes)]);
}
