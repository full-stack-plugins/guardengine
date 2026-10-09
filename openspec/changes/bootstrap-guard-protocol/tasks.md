# Tasks: bootstrap-guard-protocol

- [x] Specify v1alpha1 contract, fact, decision and report models.
- [x] Implement strict YAML contract and JSON fact parsing.
- [x] Implement generic forbidden-relation evaluator.
- [x] Bind results to deterministic contract/fact digests.
- [x] Implement CLI evaluate/verify and analyzer trait.
- [x] Implement 10 contract and CLI positive, negative, tamper and incomplete test cases.
- [x] Document local-vs-trusted-CI boundary.
- [ ] Publish independent versioned GuardEngine package. Local immutable crate/CLI preparation and independent Linux consumption are verified in [release ADR](../../../docs/local-artifact-release-adr.md); this public publication umbrella remains unchecked until a release is authorized and performed.
- [ ] Add signed attestations and protected rule baseline enforcement.
- [ ] Introduce analyzer capabilities and explicit typed fact vocabularies.
