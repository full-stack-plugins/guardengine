# Proposal: Versioned Guard integration contracts

## Why

GuardEngine main `653cebce23974114cfd75a9240f62503525f5de4` and documentation branch `97943c26d385c5108bbd26e37961575742faee27` expose a synchronous Rust library, exact `forbid_relation`, strict `guard.partme.ai/v1alpha1` objects and unsigned deterministic reports. They do not implement the reviewed run envelope, authenticated evidence eligibility or independent release adapters. Six Guards need a shared contract without moving specification, architecture, code, tests, Git or workflow policy into the engine.

This is a **planning-only** change. Source is unchanged; every new implementation task remains unchecked. Existing `bootstrap-guard-protocol` retains ownership of the implemented base protocol and its historical task ledger. This change adds integration capabilities, not a second implementation of `evaluate` or `verify_report`.

## What Changes

- Freeze a separate draft `guard.integration/v1alpha1` GuardRunEnvelope and explicit capability profiles, preserving current engine objects and native CLI behavior.
- Define generic contract validation, neutral-rule compatibility and reproducible evidence interfaces, including pre-binding errors, completed partial evaluation and execution failures.
- Add future adapters and library validation ports for immutable candidate/coverage binding and evidence references; authoritative identity, approval issuance and side effects stay in external controllers.
- Plan freshness, revocation and concurrency eligibility rules without mutating archived technical reports.
- Define pinned distribution, compatibility fixtures, gradual adoption and rollback gates. No automatic N/N-1 compatibility or optional policy backend is presumed.

## Capabilities

### New Capabilities

- `guard-integration-envelope`: a strict, separate execution envelope and capability contract.
- `guard-evidence-eligibility`: deterministic bindings and external-controller evidence eligibility.
- `guard-adapter-compatibility`: native compatibility, domain-neutral boundaries and independently pinned integration.

### Modified Capabilities

None. `guard-protocol` remains unchanged. Existing package publication/capability/signing TODOs are umbrella references: this change owns their detailed integration contract and release prerequisites; signing-provider implementation needs a later separately reviewed change. Neither ledger may be checked merely because these documents exist.

## Impact

Future surfaces include `src/integration/` (new, proposed), `schemas/integration/v1alpha1/` (new, proposed), compatibility tests and opt-in adapters. Existing `src/protocol.rs`, `src/engine.rs` and `src/main.rs` behavior must remain covered by differential fixtures. No new runtime dependency or privileged service is selected in this proposal.

Inputs: [architecture](../../../docs/architecture.md), [technical design](../../../docs/technical-design.md), [shared draft](../../../docs/integration-contract.md), [cross-project roadmap](../../guard-roadmap.md). This change is the first shared dependency; specialist discovery and native fixture work can run in parallel. GE-CONTRACT precedes integrated producer schemas, GE-ADAPTER precedes interoperability, GE-TRUST precedes authoritative eligibility, and GE-RELEASE precedes independent production distribution.

## Risks and unresolved decisions

Freeze canonical serialization, error transport and capability profiles before implementation; select identity/approval provider ports without embedding credentials or domain policy. Limits, storage retention, package publishing and future attestation/OPA backends need explicit reviewed decisions. Defaults are strict parsing, no execution of policy code, no remote side effects, exact version matching and no authorization from unsigned reports.
