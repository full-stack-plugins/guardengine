## ADDED Requirements

### Requirement: Eligibility SHALL bind immutable inputs and current authority

Generic eligibility checks MUST compare repository, requirement scope, candidate/base/merge group, analyzed snapshot, baseline, protected contract, analyzer version and required coverage. Approval expiry, revocation or unverifiable authority MUST invalidate affected authorization. Digests or claimed producer identity MUST NOT substitute for authenticated controller records.

#### Scenario: Policy changes while content stays equal
- **WHEN** the protected contract digest differs from the digest bound to a stored result
- **THEN** the old result remains an audit record but is ineligible for the current obligation

#### Scenario: Approval provider is unavailable
- **WHEN** a required approval's current scope or revocation status cannot be verified
- **THEN** authorization remains unavailable and a cached positive response cannot silently satisfy the gate

### Requirement: Concurrent attempts SHALL preserve independent histories

Attempt records and artifacts MUST remain immutable. Publication MUST compare the expected current binding; late results MUST NOT overwrite newer candidates. Reuse MUST require equal input and coverage keys and MUST NOT bypass fresh authority checks. Different requirement scopes MUST NOT cross-satisfy each other.

#### Scenario: Old PASS arrives after new BLOCK
- **WHEN** an old candidate completes after the current candidate has blocked
- **THEN** the old completion is appended only to its own history and the current result remains unchanged

#### Scenario: Two requirements share extraction
- **WHEN** two tasks have equivalent source extraction but different approval or obligation scopes
- **THEN** only the extraction artifact may be reused and each task receives independent eligibility evaluation

### Requirement: Evidence validation SHALL preserve the trust boundary

Evidence interfaces MUST distinguish consistency recomputation from source authentication and action authorization. The generic library MUST NOT issue approvals, own signing credentials or import domain policy. Audit records MUST retain binding, artifact digests, authenticated record references and invalidation causes under bounded access and retention rules.

#### Scenario: Local agent fabricates consistent unsigned evidence
- **WHEN** an uploaded report recomputes but lacks trusted producer/candidate provenance
- **THEN** consistency may be recorded but the evidence does not satisfy an authoritative CI obligation

#### Scenario: Domain policy is offered as an engine extension
- **WHEN** an integration proposes workflow stages or Git merge rules inside the engine core
- **THEN** those policies remain with their specialist owner and only neutral validated evidence crosses the interface
