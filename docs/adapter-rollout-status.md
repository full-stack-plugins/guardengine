# All-participant local rollout status

Task 4.3 inventory updated 2026-10-09 from independent review records. GE remains
22/24: tasks 4.3 and 4.6 are unchecked. SG, CG, TG and GG now have accepted local
rollout evidence; the remaining all-participant gap is AG/FG owner routing and
rollback behavior, followed by aggregate acceptance. This is neither a new GE
product/phase implementation nor a production-host deployment assertion.

| Owner | Fixed implementation and acceptance evidence | Remaining boundary |
| --- | --- | --- |
| GE | Existing `evaluate_eligibility` and append/CAS ports; migration example `68f3c1f`, independently accepted as part of 4.6 (`ge-migration-independent-review.md`) | GE does not schedule specialists. All-participant rollout, final release/evidence reconciliation and archival still depend on the remaining owner exercises. |
| SG | Real CLI/MCP six-run rollout `367cc4b`; `specguard-rollout-independent-review.md`; registration `a6abea6`, final local roadmap registration `ac580fd643e36a35bdfdfbb1e5001f3a47745214` (30/30) | Actual discrepancy blocks enable, explicit opt-in is required, either missing adapter blocks required local consumption, history/source bytes remain intact. Local access/fixture authority, cooperative host-token cancellation and in-memory history are not hosted enforcement or production identity. |
| AG | Existing native check/report, opt-in evidence and attempt history; current accepted checkpoint `583e0600781a931f79aed828e4f9d73d726cb477` | No complete four-stage owner selection/rollback exercise. Existing Rust core is not an approved new skills-first/plugin architecture. Resolve the outstanding owner/core plan and implement the actual missing local routes without inventing a GE domain adapter. |
| CG | Required-consumption rollback `9337cf2b8b8163c29abd7130c85529ec19bc0881`, `codeguard-rollback-independent-review.md`; final evidence reconciliation `2ba05c6`, `codeguard-release-index-independent-review.md`; registration `65403d3dd6942d9f862101e5902a1da8d3eccfc2` (26/26 local adapter tasks) | Actual CLI projection and explicit `Some` consumption query fixture authority; disabled `None` rejects missing required evidence, and revoked authority still denies after re-enable. Native interface/version and original bytes stay unchanged. Retained native replay is not a fresh scan, all-language qualification or hosted deployment. |
| TG | Actual 448→a619→448 tool selection, GG/TG commands and authority/original/current checks on both versions; `cc74197678302c652f0ae8177001fe0455a3d54e`, `testguard-opt-in-independent-review.md`, prior CI/shadow/rollout records | Accepted local composition; the new queue slice `f09950a` separately proves real M1→M2 reconstruction and consumption (`testguard-queue-independent-review.md`). Its 39-byte observations remain explicit fixtures, not fresh native execution. Isolation/backend and actual protected-host qualification remain separate. |
| GG | Original rollout `704f5f4`; exact-M2 required-specialist dispatch `85a9743023de788d91a370601ad2b7d5a649517a`; aligned source preparer `2dd5c204120620c5c35f269089401b75ac0829ad`; acceptance registration `231bf93` (26/28) | `gitguard-required-rollout-independent-review.md` and `gitguard-package-85a9743-independent-review.md` accept local source use. Five-domain requirements remain after adapter disable; refusals cannot reach grant. Separate grant/CAS and executor rollback preserve Applied/RecoveryRequired history. No hosted enforcement or atomic future authority across provider calls is implied. |
| FG | Existing exact-candidate joint consumer and immutable store at `b3f16f2a056b4f5c51bac87fb3f1a30558d0d22d` | Original 5.6 owner-switch/rollback deliverables remain open. Concrete controller/call compatibility and product choices are documented in ledger `flowguard-remaining-decisions.md`. Actual routing, unexplained-difference refusal and rollback are still needed; joint ALLOW/expiry evidence is not promotion evidence. |

## Source compatibility and historical artifacts

The SG rollout review records GE `b0e4b445e1a20a9354d55ebb41bbedaf16ec8a74`.
CG rollback independently compared its sibling GE source/Cargo bytes to that same
pin; TG's opt-in review also used GE b0. GG's newly aligned package contains exact
GG85a9743 plus GEb0. GE68f3c1f adds documentation/example work without changing
`src/`, Cargo.toml or Cargo.lock from b0. These are supported source combinations,
not compatibility inferred from the shared `0.1.0` version string.

The CG source package at `594644d` remains immutable and lacks the later9337cf2
`consume_required` entry. Its packaged-host GEb0 probe and the earlier GE68 host
probe are distinct evidence. Likewise GG's retained df8 package does not contain
the later required-specialist caller; the independently verified85a9743 package
does. An earlier GE crate lacked `validate_approval_record`; source-bundle success
does not retroactively qualify that crate. No new public release, implicit N/N-1,
other-platform qualification or production identity is claimed by this inventory.

## Concrete aggregate remainder

AG and FG must supply their actual owner-selected advisory/shadow/explicit opt-in
and rollback paths, preserving required policy and original history. Reuse the
already accepted SG/CG/TG/GG evidence with exact pins, commands, outputs and hashes;
do not rerun native tools just to inflate an aggregate count or turn old captures
into new observations. Policy/authority unavailable must remain unavailable after
adapter shutdown. These requirements need no new GE orchestration platform.

Task 4.4's original local queue/concurrency/expiry scope is already accepted;
its evidence does not close 4.3. The independently accepted
[migration/retention guide](migration-and-retention.md) and example are a 4.6
partial deliverable. Full4.6 additionally needs completed4.3, final supported
source/tool/version reconciliation, release acceptance and honest bootstrap-TODO
reconciliation before archival. Signing/capability/publication umbrellas remain
separate and must not be checked without their own implementation evidence.

All named independent reports are retained in the shared implementation ledger;
this update is documentation-only and performs no new runtime or matrix test.
