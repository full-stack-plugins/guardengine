use guardengine::{
    API_VERSION, AnalyzerIdentity, Completeness, ContractMetadata, ContractSpec, Enforcement,
    GuardAssertion, GuardContract, GuardFact, GuardFacts, GuardRule, GuardSubject,
};
use std::{fs, process::Command};

fn prepare_fixture() -> (tempfile::TempDir, String, String, String) {
    let dir = tempfile::tempdir().unwrap();
    let contract = GuardContract {
        api_version: API_VERSION.into(),
        kind: "GuardContract".into(),
        metadata: ContractMetadata {
            id: "cli-example".into(),
            revision: "1".into(),
        },
        spec: ContractSpec {
            rules: vec![GuardRule {
                id: "ARCH-101".into(),
                description: "prohibited".into(),
                enforcement: Enforcement::Enforce,
                assertion: GuardAssertion::ForbidRelation {
                    subject: "job".into(),
                    predicate: "depends_on".into(),
                    object: "saas".into(),
                },
            }],
        },
    };
    let facts = GuardFacts {
        api_version: API_VERSION.into(),
        kind: "GuardFacts".into(),
        analyzer: AnalyzerIdentity {
            id: "test.analyzer".into(),
            version: "1".into(),
        },
        subject: GuardSubject {
            id: "test".into(),
            snapshot_digest: "sha256:fixture".into(),
        },
        completeness: Completeness::Complete,
        facts: vec![GuardFact {
            subject: "job".into(),
            predicate: "depends_on".into(),
            object: "saas".into(),
            source: "Cargo.toml".into(),
        }],
        diagnostics: vec![],
    };
    let contract_path = dir.path().join("contract.yaml");
    let facts_path = dir.path().join("facts.json");
    let report_path = dir.path().join("report.json");
    fs::write(&contract_path, serde_yaml::to_string(&contract).unwrap()).unwrap();
    fs::write(&facts_path, serde_json::to_vec(&facts).unwrap()).unwrap();
    (
        dir,
        contract_path.to_str().unwrap().into(),
        facts_path.to_str().unwrap().into(),
        report_path.to_str().unwrap().into(),
    )
}

fn call(command: &str, contract: &str, facts: &str, report: &str) -> i32 {
    Command::new(env!("CARGO_BIN_EXE_guardengine"))
        .args([
            command,
            "--contract",
            contract,
            "--facts",
            facts,
            "--report",
            report,
        ])
        .status()
        .unwrap()
        .code()
        .unwrap()
}

#[test]
fn cli_evaluate_and_verify_preserve_block_verdict() {
    let (_temp, contract, facts, report) = prepare_fixture();
    assert_eq!(call("evaluate", &contract, &facts, &report), 2);
    assert_eq!(call("verify", &contract, &facts, &report), 2);
}

#[test]
fn cli_verify_rejects_modified_report() {
    let (_temp, contract, facts, report) = prepare_fixture();
    assert_eq!(call("evaluate", &contract, &facts, &report), 2);
    let mut altered: serde_json::Value =
        serde_json::from_slice(&fs::read(&report).unwrap()).unwrap();
    altered["decision"] = "ALLOW".into();
    fs::write(&report, serde_json::to_vec_pretty(&altered).unwrap()).unwrap();
    assert_eq!(call("verify", &contract, &facts, &report), 4);
}
