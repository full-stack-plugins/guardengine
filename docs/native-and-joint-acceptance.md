# Native compatibility and local joint acceptance reconciliation

Current status, 2026-10-10: GE4.3/4.6 are accepted for the exact local profiles in the [final rollout/release record](reviews/local-rollout-release-acceptance-20261010.md), bringing GE to24/24. The dated22/24 discussion below records the earlier acceptance state; its pending AG/FG and4.6 statements are superseded by that final evidence. GE2.1's historical timing exception remains unchanged.

Reviewed and accepted for tasks 2.1/4.4, 2026-10-09. GE source `b0e4b445e1a20a9354d55ebb41bbedaf16ec8a74`.
This evidence-only change adds no runtime implementation, changes no specialist
repository, reruns no producer/matrix, and its follow-up registration checks only tasks 2.1/4.4 under the explicit ruling below.
The [machine-readable index](evidence/2026-10-09-native-and-joint-reconciliation.json)
pins original evidence paths and SHA-256 values; those retained ledger files are
required for independent reinspection. The index is not a self-contained release
archive or an assertion that the original commands ran again today.

## Remaining-task classification

| Task | Local evidence/code state | Remaining acceptance |
| --- | --- | --- |
| 2.1 | Existing `tests/adapter_compatibility.rs` covers actual GE evaluate/verify 0/2/3/4, stdout/file destinations and BLOCK verify exit 2. Fixed old/new GE+AG comparison supplies native ArchGuard fixtures and byte parity. No functional code gap identified. | Root review may accept the retrospective functional substitute with the chronology deviation recorded below; original pre-change timing cannot be reconstructed. |
| 4.3 | Existing joint capture preserves all five specialist artifacts and FG consumer history; separate specialist rollout evidence exists. It does not demonstrate one advisory → shadow → opt-in enforcement → rollback progression for all participating adapters. | SG/CG/TG/GG local owner exercises are now independently accepted; AG/FG actual routes/rollback and their outstanding decisions remain. See [current rollout pins and boundaries](adapter-rollout-status.md). Preserve required policy after rollback; do not infer completion from 4.4 or create a GE orchestration platform. |
| 4.4 | Existing independently reviewed FG joint run used this exact GE source, two separate requirement closures, real rebuilt queue candidates, same-store overlapping attempts and late completion, expiry/revocation rejection. | Ready for GE-specific root review against the original queue/concurrency/expiry scope. Production hosted authentication, rollback and another native producer matrix are not additional prerequisites for this bounded local task. |
| 4.6 | Integration API, publication, trust, release ADR and deferred decisions already exist; old bootstrap publication/signing/capability TODOs correctly remain unchecked. | Reconcile migration examples and audit/retention operating guidance with accepted scope, then final release acceptance review; do not archive while original tasks remain incomplete. Public publishing/signing are explicitly separate, not excuses to stop local documentation. |

## 2.1: functional parity with an explicit chronology deviation

Original evidence: ledger `guardengine-native-retrospective-report.md` and
`ge-native-retrospective/{pins.json,binary-sha256.json,run_matrix.py,observations.json}`.
Fixed pre-adapter sources were GE `e9261e6164291662eb402421b5d73278c8637155`
and AG `9ed191acf3c3b1f9c0ca9edf3d0621b6a7c53f04`; compared sources were
extracted GE `c80ec325449842d51007db2fd507040c53dc8f51` and matrix AG
`c5cb98312ae18fc8702a13dae5d80dbddb6d008e` (the original ledger retains precise binary hashes). Old suites recorded
10 GE and 5 AG passes; the retrospective matrix executed 56 actual CLI calls.

For each allow/block/review/advisory/partial case, both generations ran AG check
stdout/file output and GE evaluate stdout/file plus verify. Invalid contract and
tampered verification returned 4 with empty stdout; BLOCK verification returned 2.
Five facts/report sets matched byte-for-byte across old/new source. This is fixed
source retrospective equivalence, **not** characterization captured before adapter
changes and **not** historical RED. The original task's “before any adapter change”
clause was not met and is an irrecoverable process deviation. Functional acceptance
requires that exception to be explicitly recorded; no fresh test can repair its
timestamp. This change neither silently rewrites the requirement nor marks it done.

Read-only reconciliation checked all 56 recorded statuses, required empty-stdout
hashes, two BLOCK verification exits, all ten recorded contract/facts/report digest
sets, and 15 baseline/current byte pairs (facts, report, engine-report across five
cases). It validates retained evidence, not a new current-head CLI execution.

## 4.4: reuse of independently executed joint evidence

Original accepted source was FG `1fd70a11ac0e874ca1bf21801987320c237647de`;
the narrowly corrected same-domain drift replay used FG
`7b3c3f498dd8c83797a918c0d50f3bcf199ba36f`. GE was exactly `b0e4b445…`;
full source pins and exact A/B/M1/M2 candidate/base/group/member bindings are in
the index. The fixed FG consumer and native tool hashes are retained in the
original manifest, not inferred from live repository HEADs.

- A and B carry distinct task IDs and actual SG approved requirement closures.
- M1/M2 are actual merge-tree/commit-tree candidates; M2 changes base and ordered
  members. Cross-candidate A→B, A→M1, M1→M2 attempts return exit 4/null decision.
- Four fresh groups of SG/AG/CG/TG/GG producers feed the actual FG consumer:
  A/B/M1/M2 each return exit 0/ALLOW, with execution authorization still false.
- One MemoryRunStore has overlapping A/B reservations. Late M1 remains historical
  and cannot replace already published M2; A/B remain independent. Producer
  processes were sequential: no process-concurrency or durable-store claim.
- Baseline authority expiry and review revocation each return exit 4/null decision.
  Rule/analyzer drift and shared-project invalidation are additional observed
  checks; they do not expand the original GE queue/concurrency/expiry requirement.

The prior independent review verified 366 source/Cargo files, 304 retained producer
artifacts and 18 CLI streams. Today's read-only check rehashed all 304 producer
artifacts in **both** original and corrected captures, all 18 corrected CLI streams,
and all 18 request/authority file digests named by nine actual CLI argv records.
Exact protected local fixture authority is preserved. This does not authenticate
production queue events, install protected checks, issue grants, or invoke a merge/
ref executor. Those boundaries remain explicit instead of turning fixture success
into production authority.

Root review inputs: ledger `flowguard-joint-independent-review.md`, original
`flowguard-joint-independent-capture/producer-manifest.json` and
`independent-source-and-artifact-verification.json`, corrected
`flowguard-joint-drift-independent-capture/{result.json,drift-isolation.json}`.
All are digest-pinned by the index. No rollout/rollback claim is inferred here;
that is task 4.3 and remains open.

## Independent review and root ruling

Independent review of `c5f156e4fe7f11f649c3dbad3519d9ca928b27b4` passed;
ledger `guardengine-native-joint-doc-independent-review.md` and its machine check
are digest-pinned in the index. Root accepted 4.4 for its original local
queue/concurrency/expiry scope and accepted 2.1 functional compatibility using
retrospective fixed-source verification as an explicit historical process exception.
The pre-adapter capture chronology remains unsatisfied, not retroactively rewritten.
Only these two boxes are registered; GE is 22/24, with 4.3 and 4.6 still open.
Earlier review-pending text above records the evidence submission, not the final ruling.
