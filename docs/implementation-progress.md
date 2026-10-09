# GuardEngine implementation progress

Branch `impl/guard-roadmap-20261009`; source baseline `e9261e6164291662eb402421b5d73278c8637155` (documentation + OpenSpec). Original document branch untouched. Product pushes require parent confirmation.

## Execution order

1. Preserve legacy requirements, add delta/scenario format; baseline 10 tests and old/new strict validation.
2. GE-CONTRACT tasks 1.1–1.5: explicit field/profile decisions, bounded strict model/schema and golden vectors. Tests precede code; 1.6 waits consumer mapping review.
3. GE-ADAPTER 2.1–2.5: preserve existing binary behavior, validate actual engine artifacts and safe opt-in output; 2.6 waits real consumers.
4. GE-TRUST 3.1–3.5: pure generic records/eligibility/concurrency ports without credentials/domain policy; production 3.6 requires authentic controller evidence, explicit fixtures cannot substitute.
5. GE-RELEASE local package/matrix and joint scenarios after adapters, no registry publishing or production activation authorized.

## Evidence

- Baseline `cargo test --all-targets` on Rust 1.99.0: 10 tests passed, 0 failed. Log: execution ledger guardengine-baseline.log (cloud review artifact).
- Legacy bootstrap spec normalized without changing normative paragraphs or any task status; strict validation now valid, zero issues.
- No new task marked complete yet. Decisions in schemas/integration/decisions.md require independent review.

## First contract slice (implemented, review pending)

- Closed Rust model/loader, explicit EngineBacked/NativeOnly capability, required nullable fields, binding/coverage/lifecycle/UTC invariants, byte/depth/string/collection budgets. No new CLI and no network resolution.
- JSON Schema and positive/negative vectors; schema limitations and mandatory runtime semantic validation are explicit.
- Exact artifact byte digest checking plus existing full-report recomputation and analyzer/snapshot/completeness/decision agreement. Native profiles cannot satisfy stronger evidence.
- Preserved native engine evaluation code. Golden compact typed serialization pinned separately from historical core digests.
- `cargo test --all-targets`: 16 passed (10 historical, 6 integration); `cargo clippy --all-targets -- -D warnings`: passed. Python Draft 2020-12 schema-vector test passed. RED logs show missing semantic validation and missing artifact verification failing before implementation; initial artifact fixture error was corrected before the behavioral RED run.
- Tasks 1.1–1.5 and 2.2 have substantial implementation, but reviewer approval, all named acceptance fixtures and consumer matrix remain pending. 1.6, 2.1/2.3–2.6, trust, publication and end-to-end gates are not complete. No task boxes changed.

## Independent contract review and corrections

Review of 4e881ba identified an important multiplicative recomputation budget issue. Commit36d63a5 adds conservative expansion/comparison preflight before unchanged core evaluation; the same reviewer reproduction now rejects early and peak memory fell from133MiB to8.4MiB (environment-specific observation, not capacity guarantee). A schema URI-control discrepancy was fixed, and final-newline lexical vectors were added. Final core+integration suite17tests, schema vectors and strict clippy pass. Original16tests also pass on declared MSRV Rust1.85.0.

Independent review accepted full tasks1.1 and1.2 locally; those two boxes are checked. Tasks1.3–1.5 remain partial, task2.2 behavior is accepted but its staged prerequisites remain open. The GE-CONTRACT gate is not complete, no production trust/release/queue acceptance is claimed. Review artifacts live in cloud execution ledger guardengine-contract-review.md; implementation fixes remain local and unpushed.

## Lifecycle and pure trust review checkpoint

Commits0c02178/bc812f6/799fb1e/11a5ba7 add profile vectors, required-scope preparation, ordered evidence regressions, and generic eligibility/append-only CAS ports. Independent review found and closed a lifecycle defect: required scopes freeze, actual observed/missing/status coverage is supplied at finish; weakening frozen scope rejects. Full suite35tests and clippy pass independently.

Full locally accepted tasks now1.1–1.5 and3.1–3.5 (10/24). Trust acceptance covers provider-injection contracts, explicit principal authorization, immutable comparisons, expiry/revocation and in-memory append/CAS semantics only; no production provider/durable backend/hosted authorization is asserted. GE-CONTRACT consumer matrix1.6 and real-controller3.6 remain open. Review evidence: cloud ledger guardengine-trust-review.md and guardengine-trust-report.md.

## Local publication and native compatibility slice (review pending)

Added optional private-store staging/no-clobber immutable publication, cancellation cleanup and root-replacement protection. Initial missing publisher RED recorded; additional root-replacement RED exposed unsafe pathname cleanup and is fixed. Added actual CLI/library/adapter matrix preserving 0/2/3/4, output destinations, source attribution, advisory observations and partial precedence. Full suite41tests and strict clippy pass. See integration-publication.md for private-directory assumptions, Linux validation, uncertain post-link durability and no production storage authority claim. This slice does not yet check additional task boxes; ArchGuard/CodeGuard differential evidence and consumer capability freeze remain separate.
