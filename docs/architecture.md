# GuardEngine Architecture — ADR baseline 0.1

## System boundary

Partme Guard consists of six independent specialist Guards (SpecGuard, ArchGuard, CodeGuard, TestGuard, GitGuard, FlowGuard) that can evaluate shared Guard Protocol contracts through an embedded GuardEngine. No always-on central service is required.

```text
                 Contract YAML (trusted ruleset)
                             │
       Analyzer → GuardFacts │
                  └─────┬───┘
                        ▼
                  GuardEngine
          ┌─────────────┼─────────────┐
       validate       evaluate       report
      contract/facts  generic rule    & digest
          └─────────────┼─────────────┘
                        ▼
                 GuardReport JSON
                        │
             local feedback / trusted CI
```

## Boundaries

**Contract Engine** defines versioned structured data, schema checks, duplicate ID rejection and exact protocol compatibility.

**Rule Engine** evaluates domain-neutral relations in a supplied fact set. v1alpha1 supports exact prohibition of relation triples only. It does not parse Rust, Java or TypeScript.

**Evidence Engine** sorts facts for stable digests, binds the report to an analyzer identity and manifest snapshot digest, and recomputes the entire report during verify. It deliberately does not sign or approve reports.

**ArchGuard** is the first specialized Guard. It analyzes concrete Cargo metadata into neutral relation triples. Other Guards provide their own analyzers and rules.

## ADRs

- ADR-001: six independent Guards + one embedded GuardEngine.
- ADR-002: Rust is the initial runtime; protocol representations are language-independent.
- ADR-003: public protocol is versioned, unknown properties/operators fail validation.
- ADR-004: specialized source parsing is outside GuardEngine.
- ADR-005: local validation is advisory; protected CI reanalysis is authoritative.
- ADR-006: incomplete analysis is blocking; no blind fallback to ALLOW.

## Future evolution

Add typed fact vocabularies, selector scopes, analyzer capability reports, trusted signatures, provenance, performance limits, exception governance, and language-specific adapters behind compatible protocol extensions. Any changes that modify data fields or decision semantics require versioned compatibility tests.
