# GuardEngine

[English](README.md) | [简体中文](README.zh-CN.md)

GuardEngine is a **deterministic rule, contract, and evidence evaluation library** for Partme Guard.
It does **not** parse Java/Rust source, call an LLM, authorize a Git merge, or claim that an architecture is optimal.

## Engineering kernel candidates

GuardEngine is the implementation repository for the logical GuardCore role. The existing alpha CLI keeps its behavior. The following separate library candidates are under development; their APIs do not establish production trust or execute authorized delivery actions.

| Package | Responsibility | Detail |
|---|---|---|
| guard-protocol | Closed versioned documents, strict raw JSON, JCS and lossless paths | [Protocol](crates/guard-protocol/README.md) |
| guard-contract | Draft2020-12 validation, approved rule composition and verifier readiness | [Contracts](crates/guard-contract/README.md) |
| guard-subject | Actual worktree/index/commit/merge inputs and Git object integrity | [Subjects](crates/guard-subject/README.md) |
| guard-store | Private SQLite/artifact storage, idempotency and interruption recovery | [Storage](crates/guard-store/README.md) |
| guard-plan | Approved task baselines, complete frozen obligations and explicit CAS migration | [Plans](crates/guard-plan/README.md) |
| guard-runner | Bounded native argv, aggregate output limits and managed process-tree cleanup | [Processes](crates/guard-runner/README.md) |
| guard-run-ledger | Durable local run sequences, single-flight and late-result rejection | [Run ledger](crates/guard-run-ledger/README.md) |
| guard-runtime | Registered-plan dispatch and private raw-output persistence | [Runtime](crates/guard-runtime/README.md) |

The cross-repository [delivery inventory](docs/delivery/status.md) preserves native requirements, scenarios and task ownership. Inventory consistency and local library tests do not close the full architecture roadmap. CLI/MCP, trusted authority, isolated execution and controlled delivery still require their own implementation and evidence.

## v0.1.0 / Guard Protocol v1alpha1

- **Contract Engine:** load/validate strict versioned YAML contracts.
- **Rule Engine:** evaluate exact relationship assertions over analyzer-produced facts.
- **Evidence Engine:** produce deterministic JSON reports with input digests and a reproducible evaluation ID.
- **Analyzer interface:** `GuardAnalyzer` trait, implemented by specialized Guards.
- **Fail closed:** partial analyzer facts yield `INDETERMINATE` and `BLOCK`.
- **Unsigned by design:** local reports have `signed: false`; verification means recomputation, **not** trusted provenance.

## Quick start

Place the sibling [ArchGuard](https://github.com/full-stack-plugins/archguard) checkout beside GuardEngine:

```text
workspace-full-stack-plugins/
├── guardengine/
└── archguard/
```

From `archguard` (PowerShell or Bash):

```sh
cargo run -- check --project fixtures/forbidden --contract examples/agent-job-contract.yaml --facts facts.json --report evidence.json
# Expected exit code: 2 (BLOCK); both files are produced before the exit.

cd ../guardengine
cargo run -- verify --contract ../archguard/examples/agent-job-contract.yaml --facts ../archguard/facts.json --report ../archguard/evidence.json
# Expected exit code: 2: evidence is consistent but the rule BLOCKS.
```

Alternatively generate facts from any `GuardAnalyzer` and call:

```sh
guardengine evaluate --contract contract.yaml --facts facts.json --report report.json
guardengine verify --contract contract.yaml --facts facts.json --report report.json
```

Exit codes: `0=ALLOW`, `2=BLOCK`, `3=REQUIRE_APPROVAL`, `4=invalid input/verification failure`.

## Guard Protocol

| Component | v1alpha1 format |
|---|---|
| Contract | YAML `apiVersion: guard.partme.ai/v1alpha1`, `kind: GuardContract` |
| Facts | JSON `kind: GuardFacts`, sourced by a specialized Guard |
| Report | JSON `kind: GuardReport`, deterministic evaluations |
| Rule | Exact `forbid_relation(subject, predicate, object)` |
| Enforcement | `enforce`, `review`, `advise` |
| Rule status | `PASS`, `FAIL`, `REVIEW_REQUIRED`, `INDETERMINATE`, reserved `NOT_APPLICABLE` |
| Decision | `ALLOW`, `BLOCK`, `REQUIRE_APPROVAL` |

Unknown protocol fields, versions, and rule variants are rejected, never ignored. `advise` emits matched facts with a nonblocking advisory detail: `PASS` means the rule did not block, **not** that there was no observation.

See [architecture](docs/architecture.md), [technical design](docs/technical-design.md), [protocol contract](docs/protocol.md), [trust boundaries](docs/security.md), and [OpenSpec plan](openspec/changes/bootstrap-guard-protocol/).

### Scope and limitations

1. Only one rule operator is available; there is no arbitrary Rego/JS/Python expression execution.
2. Fact completeness is **claimed by the analyzer**. A malicious local agent can fabricate inputs. Trusted CI must independently regenerate facts using protected contract versions.
3. ArchGuard's current snapshot digest covers examined **Cargo manifests**, not the entire Git tree.
4. Reports are unsigned and not attestations. Signing, approvals, policy exception registries, Git branch control, and semantic code graphs are future integrations.
5. This source uses a local sibling path dependency until both repositories are published and the dependency is replaced by a pinned, independently released GuardEngine artifact.

## Contribute

Run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test --all-targets`.
License: Apache-2.0.

## Agent installation / 智能体接入

`crates/guard-install` is a separate shared installation library for the six Guard CLIs, independent of the evaluation kernel. See [design and usage](docs/agent-installation.md) and the single [OpenSpec change](openspec/changes/add-guard-agent-installation/).
