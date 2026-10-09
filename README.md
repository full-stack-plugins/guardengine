# GuardEngine

[English](README.md) | [简体中文](README.zh-CN.md)

GuardEngine is a **deterministic rule, contract, and evidence evaluation library** for Partme Guard.
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

See [architecture](docs/architecture.md), [protocol contract](docs/protocol.md), [trust boundaries](docs/security.md), and [OpenSpec plan](openspec/changes/bootstrap-guard-protocol/).

### Scope and limitations

1. Only one rule operator is available; there is no arbitrary Rego/JS/Python expression execution.
2. Fact completeness is **claimed by the analyzer**. A malicious local agent can fabricate inputs. Trusted CI must independently regenerate facts using protected contract versions.
3. ArchGuard's current snapshot digest covers examined **Cargo manifests**, not the entire Git tree.
4. Reports are unsigned and not attestations. Signing, approvals, policy exception registries, Git branch control, and semantic code graphs are future integrations.
5. This source uses a local sibling path dependency until both repositories are published and the dependency is replaced by a pinned, independently released GuardEngine artifact.

## Contribute

Run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test --all-targets`.
License: Apache-2.0.
