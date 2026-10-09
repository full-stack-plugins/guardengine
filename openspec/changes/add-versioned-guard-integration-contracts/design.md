# Design: Shared integration without replacing the pure engine

## Context and current evidence

`src/protocol.rs` validates strict engine objects and the exact API version; `src/engine.rs` sorts/deduplicates full facts, preserves rule/diagnostic order, hashes Serde JSON and recomputes full reports. `src/main.rs` exposes evaluate/verify; exit 0/2/3/4 represents ALLOW/BLOCK/REQUIRE_APPROVAL/error. With --report, evaluate writes a file instead of stdout; verify has stderr text and returns the original decision. `src/analyzer.rs` defines the external analyzer trait. Ten tests exist. Earlier GitHub CI succeeded at the documentation commit, but that does not implement or validate the new integration features.

The original bootstrap change has a historical non-delta spec format. This change preserves that record. New delta specs follow current OpenSpec requirements/scenarios; validation is structural, not functional completion.

## Goals / Non-Goals

Goals: executable schema and golden vectors for the shared draft; explicit native adapters; immutable evidence binding; generic eligibility primitives and measurable release gates. Non-goals: domain analyzers, Git mutations, workflow stages, approval issuance, signing keys, provider selection, an always-on daemon or changing existing v1alpha1 semantics.

## Boundary and proposed module layout

| Proposed surface | Owner / responsibility | Test boundary |
|---|---|---|
| `schemas/integration/v1alpha1/guard-run-envelope.schema.json` | Contract Engine integration schema, exact version/profile | Positive/negative schema corpus |
| `src/integration/model.rs`, `validation.rs` | Validate resolved producer, binding, coverage and artifact references | No domain parsing or I/O |
| `src/integration/adapter.rs` | Opt-in conversion contracts around existing evaluation | Native output parity fixtures |
| `src/integration/eligibility.rs` | Pure comparison of required/current binding and authenticated external records | No credential issuance/provider network calls |
| `tests/integration_contract.rs`, `integration_eligibility.rs`, `adapter_compatibility.rs` | Versioned schema, state, trust and migration fixtures | Unknown/missing/fault/stale cases fail closed |

These paths do not exist yet and are proposed organization, not implemented APIs. GE-CONTRACT fixes public type signatures and file/module decisions before downstream work. The existing evaluator remains the reference Rule Engine; integration cannot import any Guard's domain parser or policy.

## Contract and lifecycle decisions

Use existing `guard.partme.ai/v1alpha1` exactly. The draft envelope `guard.integration/v1alpha1` is external and closed; package semver, policy revision, analyzer version and schema version are separate. Required fields are defined by [shared draft](../../../docs/integration-contract.md). Unsupported schema/profile fails without fallback. Raw CodeGuard evidence can use a declared native-only profile but cannot fulfill an obligation requiring an engine report.

Before repo/task/candidate/base/producer/required coverage binding is resolved, a transport error has no GuardRunEnvelope. Do not invent an OID or fill required fields with empty strings. After binding, runStatus completed carries a decision; error/cancelled carries null. Valid partial facts can complete engine evaluation with INDETERMINATE/BLOCK; a crashed tool does not become a completed run merely because an old partial artifact exists. Engine-backed completed envelope decision equals its referenced GuardReport.

Attempt lifecycle queued → running → completed/error/cancelled belongs to the controller. Eligibility eligible/stale is separately derived from current bindings and approvals; it is not an additional runStatus or a mutation of stored reports. A retry creates another attempt ID and retains the original diagnostic chain.

## Rule / contract / evidence responsibilities

Contract validation owns format, version, identity uniqueness and closed schemas. Rule Engine retains exact neutral evaluation and precedence BLOCK > REQUIRE_APPROVAL > ALLOW; future operators need separate semantics/version fixtures. Evidence owns normalized inputs, artifact references, report integrity and deterministic comparison. Scope completeness originates in the analyzer and must be checked against frozen controller obligations; empty facts alone never prove coverage.

The controller authenticates producer and approver, validates action/scope/expiry/revocation, freezes protected policy, and authorizes any action. Engine eligibility APIs accept already authenticated records through typed ports; a caller-supplied boolean is not authentication. No layer rewrites a specialist REQUIRE_APPROVAL as ALLOW after approval. A FlowGuard evaluation may create its own separately scoped report while preserving all upstream artifacts.

## Baseline, concurrency and queue algorithm

Freeze repo/task/worktree/requirement set, candidate/base/merge group, source snapshot, baseline, contract, producer/analyzer version and required coverage before scheduling. Reuse extraction only with equivalent immutable keys; authorization freshness is rechecked at consumption. Sort/unique requirement IDs according to the chosen envelope canonicalization, without changing current engine digest rules.

Store immutable artifacts and attempts. Compare-and-set the current binding before publication; a late completion remains history and cannot replace a new candidate result. Invalidate eligibility on any relevant content/policy/baseline/coverage/version change or approval expiry/revocation. Exact synthetic merge-group candidate must be evaluated; PR-head evidence is insufficient. GitGuard owns candidate construction and Git preconditions; FlowGuard owns composition; engine primitives do not fetch refs or schedule domain work.

## Security and operational risks

Reject unknown versions/profiles, malformed references and resource-overbudget inputs. Keep logs bounded/redacted. Evidence references resolve only through allowed read-only stores; resolving them is not arbitrary network or filesystem authority. Source content, credentials and approval keys remain outside generic library state. Select explicit size/count/depth/time limits during GE-CONTRACT; no invented benchmark or sandbox guarantee.

Signing, arbitrary expression engines, remote execution and persistent multi-tenant services remain optional future decisions. Any chosen backend requires an ADR, threat analysis and separate acceptance scope; the base SDK must remain usable offline. If a trust provider is unavailable, required authorization is unavailable, not approved.

## Migration / rollback

First capture old engine and ArchGuard fixtures, then introduce opt-in schema/adapters, shadow compare, opt-in protected integration and only then reviewed enforcement. CodeGuard retains its CLI, check_feedback, --output, native numeric statuses and repair workbench; adapter normalizes only an explicit new surface and carries original evidence. Roll back the adapter/controller independently without deleting audit records or changing current engine data. Distribution starts from pinned source; external pinned artifact release is GE-RELEASE.

## Dependencies and acceptance

GE-CONTRACT (tasks 1.x) → GE-ADAPTER (2.x) → GE-TRUST (3.x); GE-RELEASE (4.x) follows validated compatibility. Specialist native discovery and parsing need not wait. SG baseline, AG/CG/TG evidence, GG read-only candidates and FG composed gates join for END-TO-END; no circular dependency on a future Git writer.

Acceptance requires saved fixture input, expected/actual schema decision or process exit, artifact digests and authoritative candidate binding where applicable. Test unknown fields, unsupported profiles, stale files, fabricated identity, missing coverage, expiry/revocation, candidate drift, two parallel tasks and late completion. Structural OpenSpec pass is not evidence of these runtime tests.

## Open questions

1. Which canonicalization and field-limit values will be frozen for envelope v1alpha1? Current engine byte representation must remain unchanged.
2. Which exact native-only/engine-backed capability profiles and public Rust types are published?
3. How will the trusted controller authenticate records and define revocation freshness? Provider choice remains external.
4. Which registry/versioning/reproducible-release policy publishes the independent crate and CLI?
5. Which optional runtime/attestation/service work deserves separate changes? Default is deferred, not implicitly selected.
