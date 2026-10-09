# Native migration, audit and required-original retention

This guide uses existing GE APIs; it introduces no controller, database, authority
issuer, rollout flag or retention default. It advances the documentation/example
part of task 4.6. Final release acceptance, original task 4.3 and change archival
remain separate. Signing, public publishing and typed vocabulary bootstrap TODOs
remain unchecked.

## Run the concrete migration example

From the GE source checkout, with its locked toolchain:

```sh
cargo run --locked --offline --example native_evidence_migration
cargo test --locked --offline --example native_evidence_migration
```

[Example source](../examples/native_evidence_migration.rs) loads the preserved
native ArchGuard corpus through the **existing** GE evaluator, loads the separate
integration envelope, and verifies exact artifacts plus report recomputation.
It never launches a native tool or edits a source file. Its controller binding and
clock are fixed demonstration inputs, not trusted candidate authentication.

Expected JSON: `native_decision=ALLOW`, `without_authority=provider_unavailable`,
`missing_report=invalid_evidence`, `history_records=2`, original record/bytes
unchanged, recheck clears current, and `execution_authorized=false`. The unavailable
provider deliberately issues no records. A successful example exit means these
API invariants held; it does not mean a project was authorized. Do not copy the
example's policy-from-frozen-fixture construction into an untrusted upload path.

## Migration sequence and API ownership

| Existing caller step | Explicit opt-in addition | Invariant |
| --- | --- | --- |
| Native `evaluate` / `verify` and raw reports | Preserve `guard.partme.ai/v1alpha1`; store a separate `guard.integration/v1alpha1` envelope | No relabeling native CLI exits, schema or report bytes |
| Native evidence available | `load_envelope_json(bytes, EvidenceProfile::EngineBacked)` then `verify_engine_artifacts(&envelope, contract, facts, report)` | Resolve bounded bytes in caller; no automatic URI fetching or identity inference |
| Protected caller selects candidate/policy | Construct `EligibilityPolicy` from controller-owned inputs | Full binding, expected producer/version, scopes, exact contract digest, action and authorized principals cannot come from candidate configuration |
| Required gate consumption | `evaluate_eligibility(..., &provider, trusted_now, predecessor)` on every use | Real authority port, current revocation and exact originals required; ALLOW alone never qualifies |
| New attempt or recheck | `AttemptStore::advance` before work, `append` at completion, conditional `publish` | Recheck immediately invalidates current; late completion remains history and cannot replace new generation |
| Rollback caller/adapter selection | Stop scheduling that route while retaining required checks or refusing the operation | No permissive fallback, history rewrite, Git ref rollback or approval reuse |

The preserved CLI return codes are command-specific. GE evaluate/verify use
0/2/3/4; consistent BLOCK verification remains 2. CodeGuard native exit 3 has a
separate command-specific interpretation; do not map it to GE approval by number.
See [native/joint evidence](native-and-joint-acceptance.md).

## Audit and retention operating procedure

1. Controller owners explicitly choose evidence retention and audit visibility
   periods, required-original sets, tenant reader/consumer roles and the backing
   store. No value here is an organization policy default. Preserve exact native
   and integration bytes, producer/config/version pins, protected binding and
   causal identifiers under those controls.
2. Audit `EligibilityResult.audit` together with immutable evidence references.
   Its fixed codes and identity/action/binding digests omit free-form diagnostics
   and provider secrets. Keep any principal-to-digest lookup protected separately;
   hashes are neither anonymization nor authentication. Access checks belong to
   the selected authenticated controller, not the serialization routine.
3. On artifact expiry, loss, denied access or authorized deletion, stop required
   consumption. Never return a cached eligible result because the original report
   is missing. The runnable example's empty required report returns invalid evidence
   while preserving the prior native ALLOW and historical attempt.
4. Retain append-only history and causal audit references for the chosen policy;
   record availability separately from technical decisions. GE `AttemptStore`
   has no delete operation and its in-memory implementation supplies no TTL timer,
   physical erasure guarantee or durable recovery. Do not invent deletion support
   by overwriting old attempts. A selected external backend must define these
   operations and deny consumption when required material is unavailable.
5. For local optional file publication, stage into an existing owner-private root
   with `stage_attempt`, then publish without clobbering. After uncertain directory
   sync, inspect expected final bytes/digest before retry; never delete an existing
   record to force a retry. A published file is not a current/eligible pointer.
6. Recheck current publication and authority at use, including after controller
   callbacks where the owning integration requires it. Protect clock, policies,
   ancestry, tenant boundaries and audit capacity. Storage failure or missing
   required audit must not silently turn a denial into permission.

GE owns the neutral eligibility/audit and attempt ports, not a storage or tenant
implementation. The existing FlowGuard local tenant audit profile at
`045987c9634af7b9e1b1b1011f30c9e9bf54b9f6` demonstrates caller access, original-byte
availability, expiry/removal and publication rechecks; it is a separate optional
consumer, not a GE dependency or proof of production tenant isolation. Consult
`flowguard/docs/adr/audit-retention.md` in that pinned source for its exact bounds.

## Reproducibility and remaining gates

The example test was observed failing on its absent behavior, then passing after
composition of the existing APIs. Targeted native compatibility, eligibility and
attempt-store tests also pass (20 tests total including the example). Raw [RED](evidence/migration-example/red.txt), [GREEN](evidence/migration-example/green.txt), [related tests](evidence/migration-example/related-tests.txt) and [actual output](evidence/migration-example/observation.json) are retained. Production
GE `src/` and native commands are unchanged. Existing full consumer matrices were
not repeated. This example does not close all-adapter rollout, prove external
identity, change retention policy or archive the OpenSpec change. Final 4.6 review
must reconcile actual accepted 4.3/4.4 evidence and the remaining bootstrap TODOs.
