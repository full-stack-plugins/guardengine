# GuardEngine Integration Implementation Plan

**Goal:** deliver versioned, opt-in integration while preserving the deterministic existing engine and six independent Guard owners.
**Architecture:** generic Contract/Rule/Evidence primitives in GuardEngine; domain analysis in Guards; authenticated authority and side effects in external controllers.
**Tech Stack:** existing Rust 2024/Serde/sha2; new dependencies and provider choices require review, not assumed installation.
**Spec:** [envelope](specs/guard-integration-envelope/spec.md), [eligibility](specs/guard-evidence-eligibility/spec.md), [compatibility](specs/guard-adapter-compatibility/spec.md), [design](design.md), [roadmap](../../guard-roadmap.md).

Tasks 1.1 and 1.2 are locally implemented and independently accepted; 22 tasks remain pending full acceptance. Existing bootstrap work is evidence to preserve, not work to repeat or check again. Documentation/strict validation does not complete any runtime task. Implementation was subsequently authorized; evidence and remaining limits are recorded in docs/implementation-progress.md.

## Global constraints and review focus

Keep existing `guard.partme.ai/v1alpha1` closed; proposed envelope uses separate `guard.integration/v1alpha1`. Never turn partial/error into ALLOW, numeric CodeGuard 3 into approval, unsigned integrity into identity, or a prior candidate into current queue evidence. All review-focus cases are assigned below: unknown fields (1.3), unresolved binding (1.4), partial-versus-crash (2.3), late completion (3.4), native regressions (2.1/4.2).

Each implementation item includes a testable artifact. For each runtime item, first add the named failing fixture/test, implement the minimum behavior, run the focused test and required existing regression suite, and retain expected/actual results before marking completion. Proposed files do not exist yet.

## 1. GE-CONTRACT — freeze contracts before consumers

- [x] 1.1 Record `schemas/integration/decisions.md` with approved envelope field types, nullable fields, producer profiles, resource limits, canonicalization and semantic-version policy; acceptance: every field in docs/integration-contract.md is mapped or explicitly resolved, with reviewer decision references and no implicit N/N-1 claim.
- [x] 1.2 Define proposed Rust input/output types in `src/integration/model.rs` and schema in `schemas/integration/v1alpha1/guard-run-envelope.schema.json`; test `required_fields_and_profiles` accepts complete engine/native profiles and rejects missing required fields without altering existing schemas.
- [ ] 1.3 Create `tests/fixtures/integration-envelope/` vectors for valid, unknown version/field, invalid enum, native-only capability mismatch and malformed reference; test `closed_schema_rejects_extensions` rejects every negative vector before evaluation.
- [ ] 1.4 Define independent pre-binding transport diagnostics and bound-attempt lifecycle in `src/integration/validation.rs`; test `unresolved_candidate_has_no_envelope` proves no fabricated OID, scope or producer profile and error/cancelled decisions are null after binding.
- [ ] 1.5 Freeze deterministic envelope fixture serialization and artifact reference verification in `tests/integration_contract.rs`; test `golden_vectors_are_repeatable` repeats normalization and distinguishes rule/diagnostic order from unordered requirement IDs without modifying current engine digests.
- [ ] 1.6 Review/publish the GE-CONTRACT schema and named capability matrix to all six consumer changes; acceptance: each consumer's positive/negative mapping points to one frozen version and all unresolved breaking decisions block the gate explicitly.

## 2. GE-ADAPTER — opt-in interfaces and compatibility

- [ ] 2.1 Capture current evaluate/verify and ArchGuard command fixtures in `tests/adapter_compatibility.rs`; assert 0/2/3/4, --report output destination and consistent BLOCK verification exit 2 before any adapter change; keep old test suite green.
- [ ] 2.2 Implement the reviewed opt-in adapter contracts in `src/integration/adapter.rs` after GE-CONTRACT; test `envelope_report_decisions_agree` checks exact artifact digests and rejects mismatched envelope/report outcomes.
- [ ] 2.3 Add `completed_partial_vs_execution_error` fixtures for valid partial BLOCK, crash, cancellation, serialization failure and stale output files; assert runStatus/nullable decision distinctions and diagnostic-only artifact preservation.
- [ ] 2.4 Add bounded input/reference validation and atomic per-attempt output publication to the **new adapter surface**; tests `oversized_input_rejected` and `interrupted_publish_not_current` cover budgets, allowed paths and failed writes without silently changing old CLI flags.
- [ ] 2.5 Test neutral-rule parity through adapter against `evaluate`: `exact_relation_parity`, `source_preserved`, `advisory_matches_visible`, `partial_dominates`; no requirement graph, language parser or workflow policy may be imported into engine modules.
- [ ] 2.6 Run differential integration with a pinned ArchGuard producer and CodeGuard native-only fixtures; assert command-aware mapping, declared coverage and rejected stronger profiles; record GE-ADAPTER artifacts and keep specialist implementations independent.

## 3. GE-TRUST — generic eligibility ports, external authority

- [ ] 3.1 Review typed authenticated-record ports and freshness policy in `src/integration/eligibility.rs` design/API before implementation; test fixtures distinguish verified records, untrusted booleans and provider unavailable, with no key issuance or domain approval logic in the engine.
- [ ] 3.2 Implement immutable required/current-binding comparisons; `changed_binding_is_stale` independently varies candidate, base, group, snapshot, baseline, contract, analyzer and coverage, each invalidating affected eligibility while preserving artifacts.
- [ ] 3.3 Implement approval-record scope/expiry/revocation checks against externally authenticated records; `approval_cannot_rewrite_report` preserves REQUIRE_APPROVAL and rejects stale/unverifiable authority or approval used to waive incomplete analysis.
- [ ] 3.4 Define append-only attempt storage port and compare-and-set publication contract; `late_pass_does_not_replace_current_block` plus two-requirement tests prove isolation and deterministic retry/deduplication keys without selecting a storage backend implicitly.
- [ ] 3.5 Add audit serialization/redaction/access requirements and tests in `tests/integration_eligibility.rs`; assert records bind digests/identity/action/cause, secrets are absent, and expired or missing required artifacts never qualify solely by a cached green status.
- [ ] 3.6 Run a trusted-controller integration fixture with protected policies and exact synthetic merge candidate; forged producer, candidate drift, revoked authority and provider failure all deny required eligibility; record GE-TRUST with identity adapter/version and no domain side-effect authority.

## 4. GE-RELEASE and joint acceptance — independent adoption

GE-RELEASE closes after 4.1–4.2 for explicitly supported profiles. Tasks 4.3–4.6 are later rollout/joint acceptance and do not make specialist development depend on its own completion through a release cycle.

- [ ] 4.1 Resolve publishing and dependency pinning in a release ADR, then prepare an independently versioned crate/CLI artifact with provenance and rollback instructions; acceptance: clean consumers use an immutable version without sibling checkout and existing bootstrap publication TODO references this evidence.
- [ ] 4.2 Execute supported producer/consumer matrix using frozen schema vectors, native CodeGuard parity and engine/ArchGuard regressions; unsupported combinations reject; only tested version pairs enter GE-RELEASE compatibility documentation.
- [ ] 4.3 Exercise advisory → shadow → opt-in enforcement → rollback with all participating adapters; acceptance: rollback preserves native commands, original artifacts and task history, and does not weaken required policies silently.
- [ ] 4.4 With SG-BASELINE, AG-EVIDENCE, CG-ADAPTER, TG-EVIDENCE, GG-CANDIDATE and FG-GATE available, execute END-TO-END queue/concurrency/expiry fixtures; record actual output/exit, exact candidate and authority binding rather than checklist-only success.
- [ ] 4.5 Write separate decision records for optional runtime service, attestation signing, OPA/other operators and language-neutral canonicalization changes; acceptance: each is explicitly deferred or has its own scoped change/threat model, never enabled by this integration rollout alone.
- [ ] 4.6 Complete documentation/API migration examples, audit/retention operating guidance and release acceptance review; reconcile old bootstrap umbrella TODOs only against completed evidence, leave unimplemented signing/capability work unchecked, and archive this change only after actual task evidence exists.

## Requirement-to-task traceability

| Requirement | Tasks |
|---|---|
| Integration envelopes SHALL remain separate from engine objects | 1.1–1.3, 1.6 |
| Run envelopes SHALL require a resolved immutable binding | 1.4–1.5, 3.2 |
| Execution status SHALL remain separate from technical decisions | 2.2–2.3, 3.3 |
| Eligibility SHALL bind immutable inputs and current authority | 3.1–3.3, 3.6 |
| Concurrent attempts SHALL preserve independent histories | 3.4, 4.4 |
| Evidence validation SHALL preserve the trust boundary | 3.1, 3.5–3.6 |
| Adapters SHALL preserve native command and report contracts | 2.1–2.2, 2.6, 4.2–4.3 |
| Generic evaluation SHALL preserve existing determinism | 1.5, 2.5 |
| Independent rollout SHALL require explicit compatibility evidence | 4.1–4.6 |
