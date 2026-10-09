# Threat Model / Trusted Guardrails

## Adversary

A coding agent may accidentally or intentionally change source files, local YAML policy, generated facts, report JSON, Git hooks, or its own tool invocation. No local file produced inside an untrusted workspace is authoritative evidence.

## Trust boundary

1. **Untrusted worktree:** local CLI reports are useful feedback; they do not authorize a merge.
2. **Protected rule baseline:** trusted CI loads policy from a protected revision, not a pull request's mutable copy.
3. **Trusted executor:** CI independently runs the analyzer; it does not accept user-uploaded fact sets as authoritative.
4. **Candidate SHA:** authoritative verification must record the precise checkout/tree under validation and rerun when merge base changes.
5. **Approvals:** exception approval must come from an authenticated role external to the coding agent.
6. **Report signatures:** not implemented in v0.1; `signed=false`. Recalculation is not attestation.

## Failure handling

- Invalid YAML, unknown fields and protocol versions: exit 4 (cannot evaluate).
- Analyzer cannot inspect a workspace: facts `partial`, rule `INDETERMINATE`, global `BLOCK`.
- Invalid or mismatched report: exit 4.
- Permissive local flags must not weaken protected CI controls.

## Limitations (explicit)

The ArchGuard v0.1 analyzer is bound to Rust Cargo workspace `Cargo.toml` data only. It does not inspect the entire source tree, transitive runtime behavior, features activated at runtime, external code, or API method calls. Manifest digests are **not** full repository/commit digests.

Potential additions: signed attestations (in-toto), registry/policy RBAC, immutable evidence store, bounded plugin sandbox, semantic code graph, and controlled exception expiry.


## Current implementation gaps and target controls

The current CLI accepts caller-provided filesystem paths and uses non-atomic `fs::write`; it does not enforce path containment, size/count/time budgets, credential isolation or evidence retention. The manual flag parser is not a strict unknown-flag validator. These controls must be supplied by the caller today and are proposed hardening work, not implemented protections.

A baseline revision string, analyzer identity string or snapshotDigest is not authenticated by the engine. Approval issuance, role verification, expiry/revocation and immutable candidate/base/merge-group binding belong to a trusted controller. Approval cannot make partial analysis complete. Keep signing credentials and protected policy outside candidate-controlled worktrees; preserve reports unchanged and attach any future attestation separately.

See the [shared integration draft](integration-contract.md) for invalidation, parallel requirement isolation, merge queue checks and proposed audit records, and the [technical design](technical-design.md) for precise current behavior. Those target controls do not alter current v1alpha1 fields.
