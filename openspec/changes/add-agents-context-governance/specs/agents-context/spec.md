## ADDED Requirements

### Requirement: Distinguish official format from organization conventions
SpecGuard SHALL identify six-guard-context/v1 as an organization profile and SHALL NOT claim AGENTS.md requires official headings or tool blocks.

#### Scenario: Valid Markdown without Guard blocks
- **WHEN** AGENTS.md contains ordinary project instructions without the six blocks
- **THEN** the organization check returns BLOCK with missing-block findings, not an official-format violation

### Requirement: Check real instruction content
SpecGuard SHALL check existence, balanced unique blocks, five shared instruction obligations per Guard, and optional additive project text obligations. Examples in code, quotes and comments SHALL NOT satisfy obligations.

#### Scenario: Document repair and recheck
- **WHEN** missing content is repaired using generated current instructions and the check is rerun
- **THEN** the new check analyzes current bytes, returns their SHA256 and ALLOW only if all document obligations are met

### Requirement: Fail closed within explicit coverage
SpecGuard SHALL report incomplete analysis separately from document violations; unsupported policy, unsafe paths, invalid UTF-8 and over-budget files SHALL NOT pass. Root document checks SHALL NOT claim inherited instructions or runtime execution were verified.

#### Scenario: Unsupported project policy
- **WHEN** a project policy has unknown fields or version
- **THEN** the result is INDETERMINATE with nonzero exit status

### Requirement: Share CLI and MCP behavior without mutation
CLI and MCP SHALL call the same check, and template generation SHALL only return text. Installation SHALL render the same organization obligations while retaining tool-specific rules.

#### Scenario: MCP inspection
- **WHEN** specguard_check_agents is called with an absolute project path
- **THEN** it returns the same result as agents check and leaves workspace and host configuration unchanged
