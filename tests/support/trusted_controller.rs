//! Test-only protected process/controller. No host authentication or domain executor.
use guardengine::integration::{eligibility::*, *};
use serde::Serialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    process::Command,
};
pub const ADAPTER: &str = "ge.test.local-controller-identity/v1";
pub const NOW: i64 = 1_791_540_002;
const PRODUCER: &str = "fixture-controller://local/producer";
const REVIEWER: &str = "fixture-controller://local/reviewer";
pub fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}
#[derive(Debug, PartialEq, Serialize)]
pub struct Snapshot {
    refs: String,
    head: String,
    index: String,
    worktree: String,
    protected_policy: String,
}
pub struct GitFixture {
    home: tempfile::TempDir,
    pub candidate: String,
    pub base: String,
    pub feature: String,
    contract: Vec<u8>,
}
impl GitFixture {
    pub fn new() -> Self {
        let home = tempfile::tempdir().unwrap();
        std::fs::create_dir(home.path().join("repo")).unwrap();
        let contract = serde_json::to_vec(&json!({"apiVersion":guardengine::API_VERSION,"kind":"GuardContract","metadata":{"id":"protected-controller-policy","revision":"1"},"spec":{"rules":[{"id":"fixture-review","description":"fixture-only generic relation","enforcement":"review","assertion":{"type":"forbid_relation","subject":"candidate","predicate":"requires","object":"review"}}]}})).unwrap();
        std::fs::write(home.path().join("protected-contract.json"), &contract).unwrap();
        let mut repo = Self {
            home,
            candidate: String::new(),
            base: String::new(),
            feature: String::new(),
            contract,
        };
        repo.git(&["init", "-q"]);
        repo.write("review.txt", b"ordinary\n");
        repo.git(&["add", "review.txt"]);
        repo.git(&["commit", "-qm", "base"]);
        let ancestor = repo.text(&["rev-parse", "HEAD"]);
        repo.git(&["checkout", "-qb", "target"]);
        repo.write("target.txt", b"target-only change\n");
        repo.git(&["add", "target.txt"]);
        repo.git(&["commit", "-qm", "target"]);
        repo.base = repo.text(&["rev-parse", "HEAD"]);
        repo.git(&["checkout", "-qb", "feature", &ancestor]);
        repo.write("review.txt", b"requires review\n");
        repo.write(".guard-policy.json", b"{\"allowEverything\":true}\n");
        repo.git(&["add", "review.txt", ".guard-policy.json"]);
        repo.git(&["commit", "-qm", "feature"]);
        repo.feature = repo.text(&["rev-parse", "HEAD"]);
        let tree = repo.text(&["merge-tree", "--write-tree", &repo.base, &repo.feature]);
        assert_eq!(tree.len(), 40);
        repo.candidate = repo.text(&[
            "commit-tree",
            &tree,
            "-p",
            &repo.base,
            "-p",
            &repo.feature,
            "-m",
            "synthetic merge fixture",
        ]);
        repo.verify_merge();
        repo
    }
    fn root(&self) -> std::path::PathBuf {
        self.home.path().join("repo")
    }
    fn write(&self, name: &str, bytes: &[u8]) {
        std::fs::write(self.root().join(name), bytes).unwrap();
    }
    pub fn git(&self, args: &[&str]) -> Vec<u8> {
        let out = Command::new("/usr/bin/git")
            .arg("-C")
            .arg(self.root())
            .args(args)
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("HOME", self.home.path())
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_OPTIONAL_LOCKS", "0")
            .env("GIT_AUTHOR_NAME", "ControllerFixture")
            .env("GIT_AUTHOR_EMAIL", "fixture@example.invalid")
            .env("GIT_COMMITTER_NAME", "ControllerFixture")
            .env("GIT_COMMITTER_EMAIL", "fixture@example.invalid")
            .env("GIT_AUTHOR_DATE", "2026-10-09T10:00:00Z")
            .env("GIT_COMMITTER_DATE", "2026-10-09T10:00:00Z")
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "Git fixture {:?}: {}",
            args,
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(out.stdout.len() <= 1024 * 1024);
        out.stdout
    }
    fn text(&self, args: &[&str]) -> String {
        String::from_utf8(self.git(args)).unwrap().trim().to_owned()
    }
    fn verify_merge(&self) {
        let raw = self.text(&["cat-file", "-p", &self.candidate]);
        let parents: Vec<_> = raw
            .lines()
            .filter_map(|l| l.strip_prefix("parent "))
            .collect();
        assert_eq!(parents, vec![self.base.as_str(), self.feature.as_str()]);
        assert_eq!(
            self.git(&["show", &format!("{}:target.txt", self.candidate)]),
            b"target-only change\n"
        );
    }
    fn source(&self) -> (String, Vec<u8>) {
        self.verify_merge();
        let tree = self.text(&["rev-parse", &format!("{}^{{tree}}", self.candidate)]);
        let bytes = self.git(&["show", &format!("{}:review.txt", self.candidate)]);
        (digest(&serde_json::to_vec(&(tree, &bytes)).unwrap()), bytes)
    }
    pub fn snapshot(&self) -> Snapshot {
        let files: Vec<(String, Vec<u8>)> = self
            .git(&["ls-files", "-z"])
            .split(|b| *b == 0)
            .filter(|b| !b.is_empty())
            .map(|name| {
                let name = String::from_utf8(name.to_vec()).unwrap();
                let bytes = std::fs::read(self.root().join(&name)).unwrap();
                (name, bytes)
            })
            .collect();
        Snapshot {
            refs: digest(&self.git(&["for-each-ref", "--format=%(refname) %(objectname)"])),
            head: digest(&self.git(&["rev-parse", "HEAD"])),
            index: digest(&std::fs::read(self.root().join(".git/index")).unwrap()),
            worktree: digest(
                &serde_json::to_vec(&(
                    files,
                    self.git(&["status", "--porcelain=v1", "--untracked-files=all"]),
                ))
                .unwrap(),
            ),
            protected_policy: digest(
                &std::fs::read(self.home.path().join("protected-contract.json")).unwrap(),
            ),
        }
    }
}
#[derive(Clone)]
pub struct Evidence {
    pub envelope: GuardRunEnvelope,
    pub contract: Vec<u8>,
    pub facts: Vec<u8>,
    pub report: Vec<u8>,
}
impl Evidence {
    pub fn artifacts(&self) -> ArtifactBytes<'_> {
        ArtifactBytes {
            contract: &self.contract,
            facts: &self.facts,
            report: &self.report,
        }
    }
}
// Only controller-owned execution below can insert an issued record. No upload/register API.
pub struct Controller {
    policy: EligibilityPolicy,
    contract: Vec<u8>,
    issued: RefCell<BTreeMap<String, ProducerRecord>>,
    approval: RefCell<Option<ApprovalRecord>>,
    producer_revoked: Cell<bool>,
    approval_revoked: Cell<bool>,
    unavailable: Cell<bool>,
    pub producer_queries: Cell<usize>,
    pub approval_queries: Cell<usize>,
}
impl Controller {
    pub fn freeze(repo: &GitFixture) -> Self {
        let contract = std::fs::read(repo.home.path().join("protected-contract.json")).unwrap();
        assert_eq!(contract, repo.contract);
        let producer:Producer=serde_json::from_value(json!({"guard":"controller-fixture","version":"1","analyzerId":"fixture.git-blob","analyzerVersion":"1"})).unwrap();
        let binding:RunBinding=serde_json::from_value(json!({"repoId":"fixture:private-repo","taskId":"controller-integration","worktreeId":"fixture-worktree","requirementIds":["R-controller"],"candidateOid":repo.candidate,"baseOid":repo.base,"mergeGroupId":digest(&serde_json::to_vec(&(&repo.base,&repo.feature)).unwrap()),"sourceSnapshotDigest":repo.source().0,"baselineDigest":null})).unwrap();
        Self {
            policy: EligibilityPolicy {
                binding,
                producer,
                required_scopes: vec!["fixture.git-blob".into()],
                contract_digest: digest(&contract),
                action: "fixture.observe-eligibility".into(),
                producer_principals: [PRODUCER.into()].into(),
                approval_principals: BTreeMap::from([(
                    "fixture.review".into(),
                    [REVIEWER.into()].into(),
                )]),
            },
            contract,
            issued: RefCell::new(BTreeMap::new()),
            approval: RefCell::new(None),
            producer_revoked: Cell::new(false),
            approval_revoked: Cell::new(false),
            unavailable: Cell::new(false),
            producer_queries: Cell::new(0),
            approval_queries: Cell::new(0),
        }
    }
    pub fn policy(&self) -> &EligibilityPolicy {
        &self.policy
    }
    pub fn approve(&self) {
        *self.approval.borrow_mut() = Some(ApprovalRecord {
            principal: REVIEWER.into(),
            purpose: "fixture.review".into(),
            action: self.policy.action.clone(),
            binding: self.policy.binding.clone(),
            contract_digest: self.policy.contract_digest.clone(),
            validity: validity(),
        });
    }
    pub fn set_revoked(&self, producer: bool, approval: bool) {
        self.producer_revoked.set(producer);
        self.approval_revoked.set(approval);
    }
    pub fn set_unavailable(&self, value: bool) {
        self.unavailable.set(value);
    }
    pub fn run_owned_job(&self, repo: &GitFixture) -> Evidence {
        let (source, bytes) = repo.source();
        assert_eq!(source, self.policy.binding.source_snapshot_digest);
        assert_eq!(repo.candidate, self.policy.binding.candidate_oid);
        assert_eq!(repo.base, self.policy.binding.base_oid);
        assert_eq!(bytes, b"requires review\n");
        let facts=serde_json::to_vec(&json!({"apiVersion":guardengine::API_VERSION,"kind":"GuardFacts","analyzer":{"id":"fixture.git-blob","version":"1"},"subject":{"id":"fixture:private-repo","snapshotDigest":source},"completeness":"complete","facts":[{"subject":"candidate","predicate":"requires","object":"review","source":format!("git:{}:review.txt",repo.candidate)}],"diagnostics":[]})).unwrap();
        let report = guardengine::integration::evaluate_bounded(
            &guardengine::load_contract_yaml(&self.contract).unwrap(),
            &guardengine::load_facts_json(&facts).unwrap(),
        )
        .unwrap();
        let report = serde_json::to_vec(&report).unwrap();
        let artifact = |name: &str, bytes: &[u8]| json!({"uri":format!("artifact://controller-job-1/{name}"),"digest":digest(bytes),"mediaType":"application/json"});
        let envelope=load_envelope_json(&serde_json::to_vec(&json!({"apiVersion":"guard.integration/v1alpha1","kind":"GuardRunEnvelope","runId":"controller-job-1","producer":self.policy.producer,"binding":self.policy.binding,"runStatus":"completed","decision":"REQUIRE_APPROVAL","coverage":{"status":"complete","requiredScopes":self.policy.required_scopes,"observedScopes":self.policy.required_scopes,"missingScopes":[]},"artifacts":{"contract":artifact("contract",&self.contract),"facts":artifact("facts",&facts),"report":artifact("report",&report),"domain":[]},"approvalRefs":["controller:review-1"],"diagnostics":[],"startedAt":"2026-10-09T10:00:00Z","finishedAt":"2026-10-09T10:00:01Z","expiresAt":null})).unwrap(),EvidenceProfile::EngineBacked).unwrap();
        verify_engine_artifacts(&envelope, &self.contract, &facts, &report).unwrap();
        assert_eq!(repo.source().0, self.policy.binding.source_snapshot_digest);
        // Test-only authenticated issuance: this path just executed the controller-owned
        // producer against its independently frozen Git object and verified bytes.
        // Digest is a lookup key for that issuance, never a signature or upload grant.
        let key = digest(&serde_json::to_vec(&envelope).unwrap());
        assert!(
            self.issued
                .borrow_mut()
                .insert(
                    key.clone(),
                    ProducerRecord {
                        principal: PRODUCER.into(),
                        producer: self.policy.producer.clone(),
                        envelope_digest: key,
                        validity: validity(),
                    }
                )
                .is_none(),
            "fixture run ID may not overwrite an issued record"
        );
        Evidence {
            envelope,
            contract: self.contract.clone(),
            facts,
            report,
        }
    }
    pub fn evaluate(&self, e: &Evidence) -> EligibilityResult {
        evaluate_eligibility(&e.envelope, e.artifacts(), &self.policy, self, NOW, None)
    }
}
fn validity() -> Validity {
    Validity {
        issued_at: NOW - 2,
        expires_at: NOW + 60,
        revoked: false,
    }
}
impl AuthorityProvider for Controller {
    fn verify_producer(
        &self,
        e: &GuardRunEnvelope,
        d: &str,
    ) -> Result<ProducerRecord, AuthorityError> {
        self.producer_queries.set(self.producer_queries.get() + 1);
        if self.unavailable.get() {
            return Err(AuthorityError::Unavailable);
        }
        if digest(&serde_json::to_vec(e).unwrap()) != d {
            return Err(AuthorityError::Untrusted);
        }
        let mut record = self
            .issued
            .borrow()
            .get(d)
            .cloned()
            .ok_or(AuthorityError::Untrusted)?;
        record.validity.revoked = self.producer_revoked.get();
        Ok(record)
    }
    fn verify_approval(&self, reference: &str) -> Result<ApprovalRecord, AuthorityError> {
        self.approval_queries.set(self.approval_queries.get() + 1);
        if self.unavailable.get() {
            return Err(AuthorityError::Unavailable);
        }
        if reference != "controller:review-1" {
            return Err(AuthorityError::Untrusted);
        }
        let mut record = self
            .approval
            .borrow()
            .clone()
            .ok_or(AuthorityError::Untrusted)?;
        record.validity.revoked = self.approval_revoked.get();
        Ok(record)
    }
}
