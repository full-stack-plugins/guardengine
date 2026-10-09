## ADDED Requirements

### Requirement: Integration envelopes SHALL remain separate from engine objects

The integration Contract Engine MUST validate an explicit closed `guard.integration/v1alpha1` GuardRunEnvelope without adding fields to current `guard.partme.ai/v1alpha1` GuardContract, GuardFacts or GuardReport. Unsupported versions and capability profiles MUST be rejected; package versions and policy revisions MUST NOT imply wire compatibility.

#### Scenario: Producer adds orchestration fields to existing facts
- **WHEN** a producer adds candidateOid or approvalRefs to a current GuardFacts object
- **THEN** the existing loader rejects the unknown fields and the adapter does not silently strip them to claim compatibility

#### Scenario: Consumer receives an unsupported integration profile
- **WHEN** a native-only producer is supplied to an obligation requiring engine-backed evidence
- **THEN** the consumer rejects the capability mismatch without manufacturing an engine report

### Requirement: Run envelopes SHALL require a resolved immutable binding

A GuardRunEnvelope MUST contain the shared contract's validated producer, repo/task/worktree/requirement, candidate/base/merge-group, snapshot/baseline and frozen coverage binding. Pre-binding errors MUST use separate transport diagnostics and MUST NOT fabricate identifiers or an envelope. Optional nullable fields MUST follow the explicit schema.

#### Scenario: Candidate cannot be resolved
- **WHEN** invocation parsing cannot resolve the required immutable candidate or base
- **THEN** the CLI/API returns a transport failure with no GuardRunEnvelope and no substituted HEAD or empty OID

#### Scenario: Queue membership changes
- **WHEN** a new merge group produces a different candidate or base
- **THEN** the old binding cannot satisfy the new required obligation and a new bound attempt is required

### Requirement: Execution status SHALL remain separate from technical decisions

Completed engine-backed envelopes MUST carry the referenced GuardReport decision unchanged. Bound error or cancelled attempts MUST carry null decision. Valid partial facts MUST retain INDETERMINATE/BLOCK semantics; a tool crash MUST NOT be disguised as a completed evaluation. A review approval MUST NOT rewrite an archived technical report.

#### Scenario: Partial evaluation completes
- **WHEN** valid partial GuardFacts with diagnostics are successfully evaluated
- **THEN** the completed envelope carries BLOCK and references the unchanged INDETERMINATE report

#### Scenario: Analyzer crashes after creating a local artifact
- **WHEN** the analyzer crashes before satisfying the frozen execution contract
- **THEN** the bound run is error with null decision and any partial artifact is retained only as diagnostic evidence

#### Scenario: Review requirement receives external approval
- **WHEN** an authenticated controller approves the exact action associated with REQUIRE_APPROVAL evidence
- **THEN** the report and its envelope remain REQUIRE_APPROVAL while the controller separately records action authorization
