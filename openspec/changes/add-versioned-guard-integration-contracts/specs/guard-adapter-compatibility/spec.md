## ADDED Requirements

### Requirement: Adapters SHALL preserve native command and report contracts

Integration MUST be opt-in and versioned. Existing GuardEngine/ArchGuard 0/2/3/4 behavior and CodeGuard command-specific exits, reports, hooks and --output behavior MUST remain unchanged. Adapters MUST interpret command, flags, run status and evidence together rather than normalize numeric codes blindly.

#### Scenario: CodeGuard normal aggregate check exits three
- **WHEN** native check_feedback records local incomplete feedback and process exit 3
- **THEN** an adapter preserves incompleteness and does not infer REQUIRE_APPROVAL merely from that number

#### Scenario: Consistent blocking report is verified
- **WHEN** current guardengine verify recomputes a BLOCK report successfully
- **THEN** the native CLI still exits 2 and adapter metadata distinguishes consistency success from the blocking decision

### Requirement: Generic evaluation SHALL preserve existing determinism

Adapters MUST preserve current exact relation evaluation, fact-record sorting/deduplication, rule and diagnostic order, evaluation-ID inputs and complete-report verification. Future canonicalization, capability or operator changes MUST use explicit versioned fixtures and MUST NOT silently alter current evidence semantics.

#### Scenario: Fact order changes but complete records are equal
- **WHEN** an adapter permutes identical complete fact records before current evaluation
- **THEN** normalized reports remain equal under the current engine version

#### Scenario: Source attribution changes
- **WHEN** equal relation triples have different source values
- **THEN** current normalization retains distinct fact records and the adapter does not erase source attribution

### Requirement: Independent rollout SHALL require explicit compatibility evidence

Pinned distribution and enforcement MUST follow schema, adapter and trust acceptance gates. Unsupported combinations MUST fail without fallback. Rollback MUST preserve native behavior and immutable history. N/N-1 support, optional expression runtimes and attestation backends MUST require separate reviewed decisions and actual compatibility evidence.

#### Scenario: Upgrade fails a required compatibility vector
- **WHEN** a proposed SDK/adapter version changes a native report or unsupported-profile outcome
- **THEN** rollout stops and the previous compatible native path remains available without deleting prior records

#### Scenario: Package release precedes complete domain implementations
- **WHEN** the engine artifact is independently released while some Guards remain incomplete
- **THEN** only proven capability profiles are advertised and missing specialist capabilities cannot be represented as ALLOW
