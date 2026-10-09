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

## Reviewed checkpoint: parity and deferred decisions

Tasks 2.5 and 4.5 accepted by independent publication review: neutral-rule differential tests preserve exact relations, sources, advisory visibility and partial dominance; optional services, signing, OPA and canonicalization changes have separate explicit deferrals. The later YAML alias P1 was corrected in 4af3853 and independently retested; this does not close unrelated adapter or release tasks. FactBudget in 0907beb has four additional regression tests and remains separately reviewed before consumer acceptance.

## Reviewed fault boundary and compatibility work

Tasks2.3/2.4 accepted after6ae9bb4a: real child crash/cancel, partial serialization sink failure, no new final successful file, old attempt preservation and explicit recovery diagnostics. Full60tests independently repeated; this is private Linux/local adapter publication, not a current pointer or supervisor. Local acceptance is now15/24.

See local-consumer-matrix.md for14 actual artifact sets and70 negative checks. This does not yet complete all-six native producer, authenticated trust, hosted enforcement or release gates. Task2.1 historical ordering caveat remains explicit; retrospective compatibility evidence is preserved rather than labelled historical RED.

## Frozen six-consumer gate: 17/24 locally accepted

Independent review of `e84b306` accepts tasks1.6 and2.2 for the explicit local profiles. All six consumer changes now point to exact schema/manifest hashes and named positive/negative mappings. The 16-case corpus (15 engine-backed sets and one expressly derived CodeGuard native-only vector) passed61 tests,63 raw-file hashes and15 extra analyzer/source-snapshot mismatch probes. No specialist logic entered the engine. Review evidence: cloud ledger `guardengine-corpus-independent-review.md`. Tasks2.6/4.2 remain partial pending actual command-aware native CodeGuard0/3/error differential mapping; a derived wire vector does not replace this. Production identity, hosted enforcement and public release remain unclaimed.

## Reviewed actual Ruff compatibility: 19/24 locally accepted

Independent review of d251a968 accepts tasks2.6 and4.2 for the exact documented local version pairs. Current corpus:34 cases; immutable extracted c80ec325 consumer:18 additional positives and90 negatives. All90 new raw artifacts match actual SDK captures, while the old manifest and63 artifacts remain unchanged. The command-aware mapping keeps original CodeGuard native exit3 separate from the narrow Ruff F401 SDK profile and rejects stronger native-only qualifications. Full suite62 tests and strict Clippy passed before independent corpus reruns. Evidence: cloud ledger guardengine-ruff-matrix-independent-review.md; maintained profile documentation: ruff-consumer-matrix.md.

This is neither a runtime producer SHA allowlist nor authenticated Git/authority evidence. Fixture candidate bindings stay synthetic; broad CodeGuard CLI coverage,57 real native tools, other platforms and untested version pairs are not qualified. Task2.1 historical timing caveat,3.6 production identity,4.3 rollout,4.4 joint scenarios and4.6 final acceptance remain open.

## Remaining native and joint evidence reconciliation (review pending)

At source `b0e4b445`, the [native/joint acceptance note](native-and-joint-acceptance.md)
and its digest-pinned index reconcile original tasks 2.1, 4.3, 4.4 and 4.6.
Read-only checks confirm 56 retained native CLI observations, 15 old/new byte pairs,
304 original producer artifacts in each of two joint captures, 18 CLI streams and
18 request/authority digests. No producer or full matrix was rerun. Task 2.1 has
functional retrospective evidence but an irrecoverable pre-change timing deviation;
task 4.4 reuses independently executed local queue/concurrency/expiry evidence with
exact GE `b0e4b445`. Both await root acceptance. Task 4.3 still needs the explicit
all-adapter rollout progression; 4.6 still needs documentation reconciliation and
final acceptance. No checkboxes changed and no production release/trust claim added.

## Accepted remaining native/joint evidence: 22/24

Root accepted independent review of c5f156e; see
[the acceptance record](reviews/accepted-2-1-4-4-20261009.md). Task 2.1 uses an
explicit retrospective functional substitute with its irrecoverable pre-change
timing deviation retained. Task 4.4 is accepted for the original local joint
queue/concurrency/expiry profile. Only these two task boxes changed. Tasks 4.3
and 4.6 remain open, as do the separate publication/signing/capability umbrellas.

## Migration example and rollout inventory (4.6 partial, independently accepted)

Added executable `examples/native_evidence_migration.rs` using only existing APIs:
native report/evidence recomputation, deliberate unavailable-authority denial,
missing required-original denial, immutable attempt history and recheck invalidation.
Behavioral RED then GREEN, 20 focused tests, actual example execution and strict
example Clippy pass; no product `src/` changes or full matrix rerun. See
[migration/retention operations](migration-and-retention.md) and
[all-participant rollout inventory](adapter-rollout-status.md). SG/CG rollout has since been independently accepted alongside TG/GG; AG/FG owner routing remains open. The fixed68f3c1f migration example/retention guide was independently accepted as a partial deliverable (ledger `ge-migration-independent-review.md`), not final4.6 acceptance. Tasks4.3/4.6 remain unchecked and this change is not archived.


## Reconciled current adapter rollout evidence (documentation only)

The [all-participant inventory](adapter-rollout-status.md) now records SG final registration `ac580fd` (30/30 local tasks), CG final registration `65403d3` (26/26 local adapter tasks), accepted TG `cc741976` and GG required-specialist/source-package acceptance `85a9743`/`2dd5c20` (registration `231bf93`). Each points to its actual independent report. SG367cc4b and CG9337cf2 close the earlier local rollout gaps; no new GE product, phase model or runtime tests were added. GE runtime/Cargo bytes at68f3c1f remain equal to reviewedb0e4b445. Historical CG594644d and GGdf8 packages keep their original contents and limitations; later accepted entries are not silently assigned to old packages.

Only AG/FG actual owner routing/rollback and their outstanding design/product decisions block the remaining all-participant local exercise. Task4.6 still requires that4.3 evidence plus final supported-version/release/documentation reconciliation before archival. GE stays22/24 with4.3/4.6 unchecked; no production identity, hosted enforcement, native sandbox, broad language/platform or public release qualification is inferred.
