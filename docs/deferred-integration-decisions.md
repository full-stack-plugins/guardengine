# Explicitly deferred integration options

These options are not enabled by the current library integration. Each requires its own scoped change, threat model, compatibility fixtures and authorization before production activation.

| Option | Decision and boundary |
|---|---|
| Hosted runtime/remote rule service | Deferred. Current evaluation is local and deterministic. A service must define tenant isolation, authentication, availability, request budgets, secret handling and data retention before it can replace local execution. |
| Attestation signing/key custody | Deferred. Hashes and recomputation prove consistency only. An identity/signing provider must define issuer authorization, audience, binding, rotation, revocation and offline verification; private keys are never added to core models or domain worktrees. |
| OPA or new rule operators | Deferred. Current neutral exact-relation semantics remain unchanged. New operators need deterministic semantics, bounded execution, explicit language/operator versions and differential tests; domain policies remain in their Guards. |
| Language-neutral canonical JSON | Deferred. Current canonical identity is pinned typed Serde serialization. Cross-language canonicalization must define Unicode/number/map ordering and migration of archived identities without silently changing existing core digests. |
| Durable shared attempt database | Deferred as a production selection. The present generic store port and in-memory CAS establish local semantics only. A durable implementation must prove crash/restart behavior, transactional generation checks, access/retention and multi-writer isolation separately. |

The external-provider/controller acceptance and protected hosted queue scenarios remain incomplete until real authority evidence exists. Labelled test providers and local stores never substitute for those acceptance gates.
