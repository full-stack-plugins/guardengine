# Specification: Guard Protocol

## ADDED Requirements

### Requirement: Strict versioning
The parser MUST reject unsupported \`apiVersion\`, unsupported \`kind\`, unknown fields and unsupported assertion variants.

#### Scenario: Unsupported protocol input
- **WHEN** the parser receives an unsupported apiVersion, kind, field or assertion variant
- **THEN** it rejects the input instead of ignoring or evaluating the unsupported content

### Requirement: Contract identity
Contract rules MUST have unique nonblank IDs, a contract ID and a revision.

#### Scenario: Duplicate or blank identity
- **WHEN** a contract includes blank metadata or duplicate or blank rule IDs
- **THEN** contract validation fails before evaluation

### Requirement: Deterministic evaluation
Given equal normalized facts, contract and engine version, evaluation ID, rule results and digests MUST remain equal.

#### Scenario: Equal normalized inputs
- **WHEN** the same contract and engine version evaluate equal normalized fact records
- **THEN** the evaluation ID, rule results and input digests are equal

### Requirement: Fail closed
When the analyzer reports \`partial\`, an enforcement gate MUST return \`BLOCK\` with \`INDETERMINATE\` rule evaluations.

#### Scenario: Partial analyzer evidence
- **WHEN** the analyzer returns valid partial facts with a diagnostic
- **THEN** each rule is INDETERMINATE and the decision is BLOCK

### Requirement: Evidence binding
Reports MUST record contract/facts digests, analyzer identity and inspected subject digest. Local reports MUST say \`signed=false\`.

#### Scenario: Unsigned local report
- **WHEN** evaluation completes for validated contract and facts
- **THEN** the report retains the input digests, analyzer and subject identity and signed is false

### Requirement: Independent verification
Verifying a report MUST recompute results from the specified contract and facts; it MUST NOT merely compare report-internal hashes.

#### Scenario: Tampered result
- **WHEN** the report decision or evaluation details differ from recomputation using the given inputs
- **THEN** verification rejects the mismatch even if self-reported digest fields were retained

