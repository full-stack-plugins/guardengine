use std::process::Command;
#[test]
#[cfg(not(feature = "test-fixture"))]
fn doctor_is_a_real_json_diagnostic_and_never_claims_missing_host_ready() {
    let out = Command::new(env!("CARGO_BIN_EXE_guardcore"))
        .arg("doctor")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3));
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value["status"], "incomplete");
    assert_eq!(value["hostProtection"], "unverified");
    let discovery = &value["result"]["discovery"];
    assert_eq!(discovery["cli"]["status"], "observed");
    assert_eq!(discovery["platform"]["status"], "observed");
    let reason = value["result"]["diagnostics"][0].as_str().unwrap();
    assert!(!reason.is_empty());
    for dimension in [
        "skills",
        "projectInitialization",
        "languageBuild",
        "git",
        "verifiers",
    ] {
        assert_eq!(
            discovery[dimension]["status"], "not_inspected",
            "{dimension}: {value}"
        );
        assert_eq!(discovery[dimension]["reason"], reason);
    }
    assert_eq!(value["result"]["sideEffects"], serde_json::json!([]));
}
#[test]
fn all_six_real_groups_are_discoverable_and_trust_override_is_rejected() {
    let out = Command::new(env!("CARGO_BIN_EXE_guardcore"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(out.status.success());
    let text = String::from_utf8(out.stdout).unwrap();
    for command in ["doctor", "contract", "plan", "run", "evidence", "policy"] {
        assert!(text.contains(command));
    }
    let out = Command::new(env!("CARGO_BIN_EXE_guardcore"))
        .args(["doctor", "--authority-config", "candidate.json"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
}
