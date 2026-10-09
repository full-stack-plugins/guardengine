# Specification: Guard Protocol

## Requirements

### Strict versioning
The parser MUST reject unsupported \`apiVersion\`, unsupported \`kind\`, unknown fields and unsupported assertion variants.

### Contract identity
Contract rules MUST have unique nonblank IDs, a contract ID and a revision.

### Deterministic evaluation
Given equal normalized facts, contract and engine version, evaluation ID, rule results and digests MUST remain equal.

### Fail closed
When the analyzer reports \`partial\`, an enforcement gate MUST return \`BLOCK\` with \`INDETERMINATE\` rule evaluations.

### Evidence binding
Reports MUST record contract/facts digests, analyzer identity and inspected subject digest. Local reports MUST say \`signed=false\`.

### Independent verification
Verifying a report MUST recompute results from the specified contract and facts; it MUST NOT merely compare report-internal hashes.
