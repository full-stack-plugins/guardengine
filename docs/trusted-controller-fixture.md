# GE-TRUST local controller integration fixture

`tests/trusted_controller_integration.rs` and its private support module exercise
actual Git objects with the existing GE eligibility ports. This is identity
adapter **ge.test.local-controller-identity/v1**, confined to a trusted local test
process. It is not a hosted identity service, signing service, OS isolation
boundary or production provider choice. No library provider or specialist
crate dependency is added.

The fixture creates an isolated repository with base, target and feature
commits. `/usr/bin/git merge-tree --write-tree` combines target and feature;
`commit-tree` creates the exact synthetic double-parent candidate without moving
refs. The helper verifies both parent OIDs and target-only blob, and reads the
candidate's actual immutable review blob. The source snapshot hashes the actual
candidate tree OID plus those blob bytes. A second actual synthetic commit with
identical tree/parents but a different commit identity demonstrates exact
candidate invalidation independently of unchanged content.

Before producing any envelope, the controller freezes its own contract bytes
(outside the candidate repository), full RunBinding, required source scope,
producer/analyzer/version, requested action and principal/purpose allowlists.
The candidate contains an intentionally weak `.guard-policy.json`; it cannot
replace the protected contract. The fixed fixture producer reads the candidate
blob, emits a generic relation, executes real `evaluate_bounded`, verifies engine
artifacts and rechecks Git source binding before private issuance. Technical
REQUIRE_APPROVAL remains unchanged even after eligibility succeeds.

The adapter's private issued-record registry has **no uploaded-report enrollment
API**. Only that controller-owned producer job can insert its issued record.
SHA256 is a lookup/integrity key for locally observed issuance, not a signature
or proof that an arbitrary principal executed code. Test-only controller approval
records use a distinct explicit principal and purpose; candidate outputs cannot
issue them. Every evaluation queries the current producer and approval records,
revocation and availability anew. A consistent rehashed but unissued envelope
is rejected as UntrustedProducer; claiming another producer cannot gain trust.

This models a local protected process boundary: whoever can modify the test
controller, registry or policy is already trusted. It does not defend against
hostile same-process code, attest external runners or select a production issuer.
`Controller` and its registration helpers exist only under tests. The fixture
clock is a controller input, not a host time attestation.

Tests cover real candidate/base/merge-group/tree-source and protected contract
drift; missing approval; producer and approver revocation; producer-port and
approval-port unavailability; expiry; and fresh recovery without reusing a cached
positive outcome. Audit output records the adapter ID/version, real Git version,
exact candidate/parents and GE audit outcome. A successful local eligibility
result contains no filesystem/ref/merge/write capability.

Before/after snapshots compare refs, HEAD, exact index bytes, tracked worktree
bytes plus status, and protected policy bytes. Eligibility evaluation has no
domain side effects. Git setup/object creation happens before these comparisons;
this is not a claim that constructing a repository itself is read-only or a full
syscall/sandbox audit. The tests exercise fixed known fixture sources only.

Run with the repository's existing target:

```
CARGO_INCREMENTAL=0 cargo test --locked --offline --test trusted_controller_integration -- --nocapture
```

Original task3.6 is a trusted-controller integration **fixture**. This provides
local GE-TRUST integration evidence for independent review; production identity,
external runner authentication and delegated domain actions still need their own
reviewed providers/controllers. No task checkbox is self-approved here.
