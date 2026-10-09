# Change: bootstrap-guard-protocol

## Why

AI coding agents need an independently executable contract verification protocol rather than self-declared correctness.

## Scope

Implement versioned Guard Protocol v1alpha1, a deterministic relation evaluator, strict contract parsing, evidence generation, CLI and analyzer trait. Prove it with a first consumer ArchGuard.

## Out of scope

Rule exceptions and RBAC, code-graph semantics, CI provenance signing, multi-language parsing, workflow orchestration, model reasoning or semantic architecture quality scoring.

## Acceptance

A compliant fact set ALLOWs, a prohibited fact set BLOCKs, partial facts BLOCK, malformed contracts do not silently pass, and report mismatches cannot verify.
