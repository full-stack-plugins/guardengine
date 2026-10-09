# Design: Guard Protocol v1alpha1

```text
contract (YAML) + analyzer facts (JSON)
                  |
         validation + normalization
                  |
        pure relation evaluator
                  |
           GuardReport JSON
```

The universal fact vocabulary begins as `(subject, predicate, object, source)`, allowing multiple analyzers to share a minimum rule operator without coupling GuardEngine to language syntax. A v1alpha1 contract cannot execute embedded code.

Evaluate every rule against a normalized fact set; aggregate verdict precedence `BLOCK > REQUIRE_APPROVAL > ALLOW`. Partial facts fail closed.

Stable hashes bind serialized contract and normalized facts. This is integrity *consistency*, not identity attestation. CI must independently regenerate facts and run with protected policy.

See `docs/protocol.md` and `docs/security.md` for schemas, invariants and the trust boundary.
