# GuardEngine

[English](README.md) | [简体中文](README.zh-CN.md)

GuardEngine is a **deterministic rule, contract, and evidence evaluation library** for six independent specialist Guards.
It does **not** parse Java/Rust source, call an LLM, authorize a Git merge, or claim that an architecture is optimal.

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

Alternatively generate facts from a `GuardAnalyzer` implementation and, after building/installing the binary on PATH, call:

```sh
guardengine evaluate --contract contract.yaml --facts facts.json --report report.json
guardengine verify --contract contract.yaml --facts facts.json --report report.json
```

Exit codes: `0=ALLOW`, `2=BLOCK`, `3=REQUIRE_APPROVAL`, `4=invalid input/runtime/verification failure`.

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

See [architecture](docs/architecture.md), [technical design](docs/technical-design.md), [shared integration draft](docs/integration-contract.md), [protocol contract](docs/protocol.md), [trust boundaries](docs/security.md), and [OpenSpec plan](openspec/changes/bootstrap-guard-protocol/).

### Scope and limitations

1. Only one rule operator is available; there is no arbitrary Rego/JS/Python expression execution.
2. Fact completeness is **claimed by the analyzer**. A malicious local agent can fabricate inputs. Trusted CI must independently regenerate facts using protected contract versions.
3. ArchGuard's current snapshot digest covers examined **Cargo manifests**, not the entire Git tree.
4. Reports are unsigned and not attestations. Signing, approvals, policy exception registries, Git branch control, and semantic code graphs are future integrations.
5. ArchGuard currently declares a local sibling path dependency on GuardEngine. Independent consumption awaits replacement with a pinned, released GuardEngine artifact.

## Implementation and target design

Reviewed source baseline: main `0284f1ef4bb93e6602d5a65a5341f10e01a63ddf` (2026-10-09). The crate and CLI exist at v0.1.0, using Rust edition 2024 with minimum Rust 1.85. Source evidence: `src/protocol.rs`, `src/engine.rs`, `src/analyzer.rs`, `src/main.rs`; 10 test functions exist under `tests/`. This documentation review did not execute them.

ArchGuard is the current embedded consumer through a sibling path dependency. SpecGuard, TestGuard, GitGuard and FlowGuard are documentation-only designs at the inspected revisions. CodeGuard retains its mature independent implementation; its future adapter must preserve existing commands, hooks, reports and exit codes.

Target integrations include authenticated baseline/approval records, expiration/revocation, isolated parallel requirement runs, exact merge-queue candidate binding and versioned run envelopes. These are **not implemented** in the engine. The draft `guard.integration/v1alpha1` envelope is separate from existing strict objects and cannot be passed to today's loaders. The historical `guard.partme.ai/v1alpha1` wire namespace is retained for compatibility; it is not a project-name prefix.

## Library and CI integration

Library callers use `load_contract_yaml`, `load_facts_json`, `evaluate` and `verify_report`, with typed errors. Specialist analyzers implement `GuardAnalyzer`; domain parsing and policy meaning stay outside the engine.

`evaluate` emits JSON to stdout only when `--report` is omitted; with that flag it writes the file. `verify` writes a consistency message to stderr, returns the report's decision code, and emits no JSON success envelope. Errors are currently plain stderr. A consistent BLOCK report therefore exits 2, not 0. Use fresh per-attempt output paths and inspect exit status; file existence alone is insufficient.

Trusted CI must obtain protected policy, independently analyze the exact candidate, verify scope and bind artifacts to that candidate. Current engine reports do not enforce branch protection, authenticate approvals or invalidate stale runs automatically. CodeGuard's existing exit codes differ and require a command-aware adapter.

## Validation status

Source, documentation links and command declarations were inspected. At the earlier architecture-review stage Cargo and OpenSpec were unavailable on PATH, so that stage claimed no functional or OpenSpec validation pass. The later planning-stage validation is recorded separately below. See the technical design for the source ledger, current limitations and measurable future acceptance gates.

## Contribute

Run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test --all-targets`.
License: Apache-2.0.


## OpenSpec implementation backlog

The incremental [proposal](openspec/changes/add-versioned-guard-integration-contracts/proposal.md), [design](openspec/changes/add-versioned-guard-integration-contracts/design.md), [requirements](openspec/changes/add-versioned-guard-integration-contracts/specs/) and [tasks](openspec/changes/add-versioned-guard-integration-contracts/tasks.md) translate the architecture into pending implementation work. See the [cross-repository dependency roadmap](openspec/guard-roadmap.md) and [structural validation record](openspec/validation-2026-10-09.md). Every new implementation task remains unchecked; this branch adds planning artifacts, not product features. Earlier source-tree inventories and validation limitations describe the inspected baseline or earlier architecture-review stage; this planning stage adds OpenSpec artifacts and separately records actual CLI validation. Existing change ownership and historical completion evidence remain intact.
