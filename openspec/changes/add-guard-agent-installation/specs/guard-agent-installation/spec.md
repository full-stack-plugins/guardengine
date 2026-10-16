## ADDED Requirements

### Requirement: Idempotent owned installation
The installer SHALL preserve user content and other tools, update only owned blocks/config entries, and support preview, synchronization and uninstall.

#### Scenario: Install twice and remove one tool
- **WHEN** two guards install to the same agent files and one is reinstalled then uninstalled
- **THEN** only one block exists per guard and the other guard and original human text remain intact

#### Scenario: Conflicting user edits
- **WHEN** an owned block or MCP entry was edited, a marker is malformed, or a target is linked
- **THEN** the operation refuses before overwriting that content

### Requirement: Separate scopes and executable installation
The CLI SHALL distinguish user/project scope, install an executable to an explicit bin directory, and never select or activate project rules as a side effect.

#### Scenario: Dry run
- **WHEN** install is invoked with dry-run
- **THEN** target instructions, configuration, executable and receipt are unchanged

#### Scenario: User scope
- **WHEN** installation targets user scope
- **THEN** the prompt has conditional tool guidance and no repository-specific branch policy

### Requirement: Truthful capabilities and shared read interfaces
Every CLI SHALL expose context and managed instructions through CLI and read-only MCP without claiming unfinished professional checks.

#### Scenario: Incomplete runtime
- **WHEN** context is queried for a guard with no professional runtime
- **THEN** the response identifies not_evaluated and no PASS or approval is fabricated

#### Scenario: MCP invocation
- **WHEN** an MCP client initializes, lists tools and calls context with an explicit project
- **THEN** the server returns the same project context as CLI and does not initialize or modify it

### Requirement: Git rules reflect active policy
GitGuard SHALL render effective rules through the existing GitFlow loader and retain its scope, activation and drift semantics.

#### Scenario: Different profiles
- **WHEN** GitHub Flow and classic GitFlow projects are queried
- **THEN** each prompt shows its own active branch roles and merge directions

#### Scenario: Candidate changed
- **WHEN** a shared policy differs from the activated policy
- **THEN** context reports unverified and does not render the candidate as approved

### Requirement: Verification
Changes SHALL include real CLI process tests, managed-file preservation tests, MCP protocol tests, and GitFlow integration tests; test fixtures SHALL remain on D or E on this host.

#### Scenario: Acceptance replay
- **WHEN** the six built CLIs are exercised in disposable project and home directories
- **THEN** the suite verifies actual file effects, protocol replies, executable copies and policy drift, and records platform limitations separately

### Requirement: Explicit execution authority
Queried project receipts SHALL NOT authorize execution of adapter programs or arbitrary binary deletion. Adapter programs SHALL come from explicit CLI options or the installed host server configuration.

#### Scenario: Forged adapter paths in a project receipt
- **WHEN** project installation metadata contains an executable or plugin path
- **THEN** a context query does not execute it, and refreshing a configured adapter requires explicit trusted options

#### Scenario: Removing a copied executable
- **WHEN** uninstall requests binary removal
- **THEN** an explicit matching bin-dir and matching owned digest are required, and the currently running executable is preserved

#### Scenario: Integration-only CodeGuard build
- **WHEN** CodeGuard is built without default features
- **THEN** its help, version and context identify integration-only capabilities and reject unavailable domain commands
