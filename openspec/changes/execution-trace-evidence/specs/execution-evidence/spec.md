## ADDED Requirements

### Requirement: Execute explicitly authorized Maven and Cargo test plans
TestGuard SHALL execute constrained native test commands only with explicit execute intent, preserve logs and fresh artifacts, and bind results to the frozen plan and unchanged project snapshot.

#### Scenario: Repair then rerun
- **WHEN** a real native test fails and its implementation is repaired
- **THEN** a new execution may pass, while the failed or stale receipt cannot verify the repaired candidate

### Requirement: Share explicitly anchored signed evidence
The shared evidence library SHALL provide deterministic source snapshots, Ed25519 signing and verification against an explicit external public key. Unsigned or imported evidence SHALL NOT be treated as trusted execution.

#### Scenario: Tampering and wrong trust anchor
- **WHEN** payload bytes, artifacts, candidate sources or the configured public key differ
- **THEN** verification rejects the receipt

### Requirement: Trace native requirements to source declarations and executed tests
SpecGuard SHALL validate declared requirement references, bounded project-local code mappings and required test IDs against current signed execution evidence without inferring semantic implementation from file existence.

#### Scenario: Missing trace or stale execution
- **WHEN** a requirement, mapped source or required test is absent or bound to an older candidate
- **THEN** trace verification cannot pass

### Requirement: Gate current evidence without granting approval
FlowGuard SHALL check the explicit plan obligations, current snapshot, signed producer, runner, required tests, artifact bytes and age. Valid technical evidence SHALL NOT grant human approval or release permission.

#### Scenario: Valid evidence awaiting approval
- **WHEN** current signed execution evidence satisfies the configured obligations
- **THEN** the result still requires the external approval authority

### Requirement: Preserve existing behavior and portable plugins
Existing imported JUnit checks, specification checks, stage prechecks, installation and architecture flows SHALL remain compatible. New verification MCP tools SHALL remain read only and share CLI implementations.

#### Scenario: Existing project without execution evidence
- **WHEN** existing read-only checks are invoked
- **THEN** their prior capabilities and explicit trust limits remain intact
