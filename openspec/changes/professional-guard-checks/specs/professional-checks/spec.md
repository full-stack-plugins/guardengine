## ADDED Requirements

### Requirement: Check active Git workflow policy without mutation
GitGuard SHALL reuse the activated GitFlow policy for branch naming, merge direction and commit checks. Missing or drifted activation SHALL NOT produce verified permission.

#### Scenario: Repair a policy violation
- **WHEN** an invalid branch, merge direction or commit message is corrected
- **THEN** CLI and MCP recompute the same policy decision without modifying Git state

### Requirement: Verify native specification structure and content drift
SpecGuard SHALL audit explicitly selected native OpenSpec or Spec Kit artifacts with bounded input and compare current inventory and content against an observation report. It SHALL NOT infer implementation or approval from structure.

#### Scenario: Native artifact changes
- **WHEN** a native specification artifact is added, removed or modified after observation
- **THEN** verification rejects stale evidence

### Requirement: Distinguish report consistency from verified test execution
TestGuard SHALL validate selected JUnit or Surefire reports against the explicit plan, required test identities and candidate/report digests. Malformed, failed, skipped, missing, duplicate or retry-masked results SHALL NOT pass. Imported reports SHALL retain execution_verified=false.

#### Scenario: A fabricated success report
- **WHEN** counters, report bytes or candidate inputs do not match the declared plan
- **THEN** the report is rejected without claiming actual test success

### Requirement: Reuse existing FlowGuard lifecycle rules safely
FlowGuard SHALL expose a bounded read-only entry to existing lifecycle policy or an explicitly limited prerequisite inspection. It SHALL preserve native specification ownership, missing evidence and approval distinctions.

#### Scenario: Missing or untrusted lifecycle adapter
- **WHEN** the required lifecycle adapter or evidence cannot be verified
- **THEN** no executable release or approval permission is granted

### Requirement: Preserve shared integration and existing architecture behavior
New domain tools SHALL share CLI and MCP implementation. Existing installation and context operations SHALL remain compatible across all six Guards, including ArchGuard Java/Maven and Cargo flows.

#### Scenario: Install all six tools into one project
- **WHEN** all six tools render managed contexts and list MCP tools
- **THEN** contexts coexist and only implemented domain tools are advertised, without a quality pass from installation
