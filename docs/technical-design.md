# GuardEngine technical design

Current local branch supplement: [implementation evidence](implementation-progress.md) records the implemented integration envelope, bounded adapter, pure trust/CAS ports and Unix publication. [Local artifact provenance](local-artifact-provenance.json) pins the independently tested Linux package. Main-baseline statements below are historical source inspection, not claims that these local modules are still unimplemented. Real identity services, signed attestations, durable production storage and hosted enforcement remain absent. Runtime dependencies additionally include pinned time, tempfile and yaml-rust2; see Cargo.toml.

Status: source-grounded current implementation plus proposed evolution. Inspected main: `0284f1ef4bb93e6602d5a65a5341f10e01a63ddf` (2026-10-09). This documentation change adds no runtime features. [Architecture](architecture.md), [wire protocol](protocol.md), [integration draft](integration-contract.md), [security](security.md).

## 1. Source and implementation ledger

| Capability | Evidence in repository | Status |
|---|---|---|
| Rust crate and executable v0.1.0, edition 2024, minimum Rust 1.85 | `Cargo.toml` | Present |
| Strict contract/facts models and validation | `src/protocol.rs` | Present |
| Neutral exact prohibition, enforcement and aggregate decisions | `src/engine.rs::evaluate` | Present |
| Deterministic digests and equality-based verification | `src/engine.rs::{digest_json,verify_report}` | Present |
| Analyzer interface | `src/analyzer.rs::GuardAnalyzer` | Present; no analyzer implemented in this crate |
| CLI evaluate/verify and exit codes | `src/main.rs` | Present; manual args, text errors |
| Contract/CLI test cases | `tests/contract_tests.rs`, `tests/cli_evidence.rs` | 10 test functions present; not run in this session |
| Signed provenance, registry, approval verification, integration envelope | No implementation in inspected source | Target only |

Dependencies are Serde/serde_json/serde_yaml, sha2 and thiserror; tempfile is a dev dependency. There is no Tokio, Clap, Rego runtime, remote policy service or persistent database in the current crate. Do not describe these as existing dependencies or committed technical choices.

## 2. Public library contracts

Current exports in `src/lib.rs`:

```rust
pub fn load_contract_yaml(bytes: &[u8]) -> Result<GuardContract, GuardError>;
pub fn load_facts_json(bytes: &[u8]) -> Result<GuardFacts, GuardError>;
pub fn evaluate(contract: &GuardContract, facts: &GuardFacts)
    -> Result<GuardReport, GuardError>;
pub fn verify_report(report: &GuardReport, contract: &GuardContract,
    facts: &GuardFacts) -> Result<bool, GuardError>;
pub fn digest_json<T: serde::Serialize>(value: &T) -> Result<String, GuardError>;
```

These signatures are interface excerpts, not standalone compilable programs. `GuardAnalyzer::analyze(&self, project_root: &Path, subject_id: &str) -> Result<GuardFacts, GuardError>` belongs to external analyzer implementations. The caller chooses project root and subject; current API does not accept cancellation/deadline/capability options. Implementations must distinguish a recoverable incomplete scan (partial facts plus diagnostics) from a failure that prevents constructing valid facts (Err).

A future library adapter should expose capability discovery and typed execution errors without changing current functions silently. Preserve exact data semantics; adapter-specific coverage or provenance belongs in the separate envelope until a new engine version is introduced. No async service or network API is needed for the initial integration.

## 3. Data validation algorithm

1. Deserialize with strict Serde fields; reject malformed YAML/JSON and unknown fields/variants.
2. Require exact API version and kind. Contract metadata ID/revision must be nonblank.
3. Require at least one rule; reject duplicate/blank IDs; reject blank assertion coordinates. Description may be absent or blank.
4. Facts require nonblank analyzer ID/version, subject ID/snapshotDigest and each fact's four strings.
5. Partial facts must carry at least one diagnostic. Complete facts may be empty and may contain diagnostics; the analyzer, not engine, determines scope completeness.
6. Revalidate in `evaluate` even when a caller constructs structs directly instead of using loaders.

Limits: identifiers are not URI-validated, digest strings are not checked for SHA-256 syntax, diagnostics are free text, and no bounded input sizes/counts are enforced. The CLI manual parser searches flag/value pairs; unknown/duplicate CLI flags are not strictly rejected. Protocol strictness must not be mistaken for strict command-line validation. Hardened parsing/limits are future work.

## 4. Evaluation and digest algorithm

Clone facts, sort/deduplicate by the whole `GuardFact` ordering (subject, predicate, object, source), then serialize validated contract and normalized facts with `serde_json::to_vec`. Compute SHA-256 as lowercase hex prefixed by `sha256:`. Hash the tuple `(API_VERSION, CARGO_PKG_VERSION, contractDigest, factsDigest)` for evaluationId.

For each ordered rule, scan normalized facts for exact subject/predicate/object equality; source is retained in matches. Partial overrides match/enforcement and yields INDETERMINATE. Otherwise map no match to PASS, enforce match to FAIL, review match to REVIEW_REQUIRED, advise match to PASS with a detail and matched evidence. Aggregate BLOCK before REQUIRE_APPROVAL before ALLOW. Emit engine version, subject/analyzer, hashes, ordered evaluations and `signed: false`.

Complexity for F facts and R rules: normalization O(F log F), matching O(R×F), plus serialized input/output size and retained match copies. In worst cases each rule retains many matching facts, so report memory can scale with R×F. A future relation index may reduce lookups but must preserve deterministic sorted match ordering and exact golden reports. Do not claim a performance benchmark or resource ceiling without measurements.

The generic digest helper does not implement a language-neutral canonical JSON specification: order of contract rules and diagnostics remains significant, and arbitrary generic map serialization need not be canonical. Target cross-language producers need pinned canonicalization fixtures and a versioned migration if bytes/semantics change. Do not sort additional fields or exclude descriptions in a compatibility adapter without a new contract.

## 5. CLI and machine-readable behavior

With Rust installed, run from this repository using current supported forms:

```sh
cargo run -- evaluate --contract contract.yaml --facts facts.json --report report.json
cargo run -- verify --contract contract.yaml --facts facts.json --report report.json
```

The paths are caller-supplied inputs; these names are not bundled fixtures. For checked-in fixtures use the sibling ArchGuard walkthrough in the README. `cargo run --` invokes the local executable; a bare `guardengine` command requires a separately built/installed binary on PATH.

| Command | Successful output | Errors |
|---|---|---|
| evaluate without --report | Pretty GuardReport JSON plus newline on stdout | Text stderr, code 4 |
| evaluate with --report | Report file, no success JSON on stdout | Text stderr, code 4; file presence alone is not success |
| verify with --report | Text consistency message on stderr; no JSON stdout | Missing/malformed/mismatched report or other error: code 4 |

Both commands return decision code 0 ALLOW, 2 BLOCK or 3 REQUIRE_APPROVAL. Thus `verify` exit 2 can mean verification succeeded for a blocking decision; code 4 means evaluation/verification could not complete. Shell wrappers must capture and interpret status explicitly, including under `set -e`.

File writes use `fs::write`, not atomic rename. A stale or partially written report may remain after failures; use a fresh per-attempt output directory and capture process status. Do not pass secrets in paths or diagnostics. Structured error envelopes, stricter argument handling, stdout mode selection and atomic publication are target work, not existing flags.

## 6. Error taxonomy and recovery

| Source | Existing result | Integration interpretation / safe response |
|---|---|---|
| Malformed YAML/JSON | GuardError::Input | Fix input; do not retry unchanged |
| Unsupported protocol/kind, invalid IDs or missing partial explanation | GuardError::InvalidProtocol | Block consumption; require supported contract |
| Serialization problem | GuardError::Serialization | Execution failure; preserve safe diagnostic |
| Missing flag/file or I/O denial | CLI boxed error, stderr, exit 4 | Correct invocation/permissions; no decision inferred |
| Recomputed report differs | Library Ok(false); CLI error/4 | Reject evidence; independently regenerate inputs/report |
| Partial analyzer scope | Valid report, INDETERMINATE/BLOCK, exit 2 | Restore required coverage; approval cannot waive incompleteness |
| Review match | Valid REQUIRE_APPROVAL, exit 3 | External trusted review process; engine report stays immutable |

Engine has no built-in retry, rollback, timeout, cancellation or durable job state. Target controller retries only transient operations with immutable input binding and a new runId, keeping each failed attempt for audit. Cancellation cannot produce ALLOW. Trust-provider unavailability is an authorization failure, not a reason to downgrade enforcement.

## 7. Target integration and responsibilities

Use [GuardRunEnvelope](integration-contract.md) as an external draft: it separates execution status from decision and carries explicit repository/task/requirement/worktree/candidate/base/merge-group binding, coverage, artifact digests and approval references. It is not accepted by current engine loaders. Domain-specific payloads remain separate referenced artifacts.

Implementation sequence: pin current engine input/output vectors; introduce an opt-in adapter around existing analyzers; preserve native behavior; add authenticated controller checks; then require protected CI evidence. ArchGuard can initially use current GuardFacts/Report directly. CodeGuard needs command-aware native report mapping and differential compatibility checks; do not replace its analyzers or hooks with engine placeholders. The other four Guards must implement actual analyzers before claiming runtime integration.

Ruleset digest and analyzer version participate in invalidation. Baseline/approval scope, expiration and revocation are checked by the trusted controller; generic engine code must not embed SpecGuard or FlowGuard policies. Parallel requirement runs have separate immutable binding and obligation sets. For merge queues, freeze required checks before running, evaluate the exact synthetic candidate, and reject stale base/group results. Gate side effects remain external and compare expected refs immediately before execution.

## 8. Security, evidence storage and extension points

Current library has no shell/network side effects; this does not sandbox analyzer processes or CLI paths. Future wrappers enforce input/output allowlists, no symlink escape, file/count/depth/time budgets, subprocess isolation and bounded logs. Policy contracts are trusted configuration but cannot execute arbitrary expressions in current protocol. LLM-generated facts are untrusted observations until independently validated.

A future evidence store should atomically publish immutable content-addressed artifacts only after successful serialization, append attempt records and derive eligibility from current policy/approval state. Do not overwrite original reports to attach signatures or approvals; use separate versioned attestations/envelopes. Signing service design, key authority, tenant isolation and retention need an ADR and threat review before implementation.

Potential generic extensions: analyzer capabilities, typed neutral vocabularies, scoped applicability, deterministic relation indexes, bounded input validation and cross-language vectors. Introduce each behind explicit version/schema and acceptance fixtures. Domain extension packs define vocabulary and extraction, never arbitrary engine access to repository credentials.

## 9. OpenSpec consistency and validation ledger

Read [bootstrap proposal](../openspec/changes/bootstrap-guard-protocol/proposal.md), [design](../openspec/changes/bootstrap-guard-protocol/design.md), [tasks](../openspec/changes/bootstrap-guard-protocol/tasks.md) and [spec](../openspec/changes/bootstrap-guard-protocol/specs/guard-protocol/spec.md). Their baseline agrees with source: strict protocol, exact relation evaluator, deterministic unsigned evidence, partial BLOCK and CLI/analyzer interface. Independent package publishing, signing/protected baseline enforcement and explicit analyzer capabilities remain unchecked future items. The new integration envelope is an additional proposal; existing checked tasks do not cover it.

This session inspected source and 10 test declarations, reviewed documentation links/commands and checked documentation diffs. Cargo and OpenSpec executables are unavailable on PATH; no compilation, functional tests or OpenSpec validation was run. Checked boxes record historical planning status, not new execution evidence.

When an implementation environment is available, the existing developer checks are:

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
```

These are commands to run, not reported passing results. A future protocol change also needs negative unknown-version/field cases, serialization golden vectors, all decision combinations, ordering/deduplication behavior, malformed/partial inputs and tamper checks. Integration tests add forged/expired/revoked approvals, candidate/base/queue drift and concurrent late completions.

## 10. Phased implementation and acceptance

| Phase | Deliverable | Measurable acceptance / rollback |
|---|---|---|
| E0: freeze current baseline | Current API/wire fixtures, command/output matrix, limitations | Actual existing tests run and archived; no changed native behavior; stop if source/fixture differs |
| E1: robust local interface | Explicit parser/error schema proposal, bounded inputs, atomic outputs | Unknown/duplicate flags rejected intentionally under documented version; interrupted output never mistaken as current result; retain old adapter during rollout |
| E2: versioned interoperability | Published pinned crate, capability matrix, draft-envelope validators and CodeGuard adapter | Supported producer/consumer matrix passes positive/negative fixtures; unsupported profile blocks; CodeGuard native parity preserved |
| E3: trusted evidence integration | Protected candidate/policy binding, authenticated approvals, immutable audit artifacts | Tamper, scope drift, expired/revoked authority and wrong merge candidate fail gates; no agent self-approval |
| E4: concurrency and safe scaling | Immutable keyed cache, queue invalidation, deterministic indexing | Parallel tasks cannot cross-satisfy, late PASS cannot override, queue changes rerun; indexed output equals reference evaluator |
| E5: optional attestation/service | Separately reviewed signing/storage/controller integrations | Key rotation, tenant permissions, recovery and retention verified; local library remains usable without service |

Before E2 schema freeze, resolve canonicalization, capabilities and distribution decisions. Before E3/E5 deployment, select approval/identity/attestation providers and retention/resource limits. These are explicit future decision gates; documentation completion does not claim these phases implemented.


### Pre-binding transport failures

The target error/cancelled envelope applies only after required invocation identity, producer profile and coverage are frozen. Invalid arguments or unresolved repository/candidate/base use separate transport diagnostics without a GuardRunEnvelope; do not invent identities or empty required fields. Current engine CLI errors remain plain stderr and exit 4. See the shared integration contract for this distinction.


## 11. Concurrent upstream documentation reconciliation

During this review, remote main advanced from the inspected source baseline to `653cebce23974114cfd75a9240f62503525f5de4` through three documentation-only commits (`dfdd215`, `3fdd3cd`, `653cebc`). No runtime source changed. This local branch merges that history, preserves both upstream README technical-design links, and keeps the complete new upstream proposal byte-for-byte at [upstream technical design snapshot](technical-design-upstream-653cebc.md). The snapshot is a historical parallel proposal, not a second normative current contract; this document and the shared integration draft explain reconciliation.

| Upstream proposal | Reconciled treatment |
|---|---|
| Frozen ValidationPlan and controlled runtime | Retain as a future controller/optional runtime integration; domain obligations remain with each Guard and the synchronous engine stays independent |
| SDK + CLI + future MCP/CI/optional service | Retained; no mandatory daemon or arbitrary shell/MCP authorization surface |
| Capabilities, resource budgets and process recovery | Retained as future acceptance requirements, not existing parser/runtime guarantees |
| Optional OPA/Rego and signed attestations | Retained as candidates requiring separate ADRs; no backend/provider selected by this documentation task |
| N/N-1 and six-Guard independent releases | Retained as future test matrices; current engine still matches exact protocol versions only |
| Upstream E0–E5 roadmap | Historical phase labels; use this document's phase table for the reconciled sequence, without treating either table as implementation completion |

The upstream snapshot's broad future input-security and runtime statements are requirements, not new implemented behavior. Full binding is required before draft error envelopes, and approved controller eligibility must not rewrite underlying engine verdicts. This merge only reconciles documentation; it does not assert runtime or OpenSpec validation.
