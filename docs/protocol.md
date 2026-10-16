# Guard Protocol v1alpha1

## Contract (YAML)

```yaml
apiVersion: guard.partme.ai/v1alpha1
kind: GuardContract
metadata: { id: agent-job-boundary, revision: "1" }
spec:
  rules:
    - id: AGJ-ARCH-001
      enforcement: enforce
      assertion:
        type: forbid_relation
        subject: agent-job
        predicate: depends_on
        object: agent-saas
```

Contract fields are strict. A `GuardContract` requires at least one rule, unique nonblank rule IDs, nonblank coordinates, and a valid enforcement value.

The first rule DSL intentionally implements **one exact triple-matching operator**. This proves the protocol without embedding architecture-specific syntax in the engine.

## FactSet (JSON)

```json
{
  "apiVersion": "guard.partme.ai/v1alpha1",
  "kind": "GuardFacts",
  "analyzer": {"id": "archguard.cargo.workspace", "version": "0.1.0"},
  "subject": {"id": "agent-job-workspace", "snapshotDigest": "sha256:..."},
  "completeness": "complete",
  "facts": [
    {"subject": "agent-job", "predicate": "depends_on", "object": "agent-saas", "source": "agent-job/Cargo.toml"}
  ],
  "diagnostics": []
}
```

A specialized analyzer MUST emit `partial` and a diagnostic when it cannot guarantee completeness within its declared analysis scope. `complete` means complete **within the analyzer's declared scope**, not that every dependency and semantic property of the program was inspected.

## Report (JSON)

Core fields: `apiVersion`, `kind`, `evaluationId`, `engineVersion`, `contractId`, `contractRevision`, `contractDigest`, `factsDigest`, `analyzer`, `subject`, `evaluations`, `decision`, `signed`.

- `enforce`: matching facts yield `FAIL`, global `BLOCK`.
- `review`: matching facts yield `REVIEW_REQUIRED`, global `REQUIRE_APPROVAL`.
- `advise`: matching facts remain in an advisory evaluation; it does not block.
- Incomplete input: every rule becomes `INDETERMINATE`; global `BLOCK`.
- If any rule blocks, global `BLOCK` takes precedence over `REQUIRE_APPROVAL`.
- `NOT_APPLICABLE` is reserved for future selective scope support.

No rule may execute shell commands or arbitrary scripts in this version.

## Exit code contract

`0 ALLOW`; `2 BLOCK`; `3 REQUIRE_APPROVAL`; `4 malformed input, unsupported version or failed recomputation`.

## Verifying evidence

`guardengine verify --contract ... --facts ... --report ...` recalculates all rule results and digests. A successful consistency check is **not** a cryptographic signature or proof that the analyzer ran on the expected Git revision. Trusted CI must rerun analysis and bind the result to its protected checkout.

## v1alpha2 graph constraints (engine 0.2.0)

The versioned extension adds `forbid_path`, `allowed_dependencies`, `require_relation`, and `acyclic` alongside `forbid_relation`. New operators require v1alpha2 contracts. All operators honor enforce/review/advise; incomplete or uncovered input always blocks.

v1alpha2 facts require:

```json
"coverage": {
  "entities": ["io.demo:app", "io.demo:domain"],
  "predicates": ["depends_on"],
  "scope": "Maven reactor declared dependencies across all declared scopes"
}
```

Inventory includes isolated modules. Facts must fall within the inventory and supported predicates. Missing entities, unsupported predicates, or legacy facts without coverage produce INDETERMINATE for v2 contracts. Malformed v2 coverage is rejected as invalid input. v1alpha1 contracts retain legacy exact-triple behavior, including absent entity matches; upgrade explicitly to enable strict coverage.

Facts and coverage inventories are sorted for stable digests. Input limits: 1,000 rules, 100,000 facts, 10,000 covered entities. forbid_path returns a deterministic shortest witness path; acyclic returns one real closed witness path. AllowedDependencies checks direct outgoing edges against objects (empty permits none); RequireRelation fails when the exact required edge is absent. Coverage is analyzer-declared scope, not an assertion of whole-program completeness.

Report apiVersion follows the contract. Reports bind engine and analyzer versions, so regenerate evidence after upgrades. Rust GuardFacts now includes optional coverage; struct-literal clients must provide it. Existing JSON/YAML v1alpha1 readers remain accepted.

ArchGuard's `verify --project ... --report ...` reanalyzes current build inputs. GuardEngine's `verify --facts ...` checks consistency of provided evidence only. Neither performs signing or grants approval.
