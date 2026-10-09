# Integration schema v1alpha1

Local implementation candidate, pending independent review and consumer matrix. No registry/schema endpoint has been published. The schema `$id` identifies this document; it is not a claim that a hosted endpoint exists.

Use `guardengine::integration::load_envelope_json(bytes, EvidenceProfile)` and, for completed engine-backed records, `verify_engine_artifacts(&envelope, contract_bytes, facts_bytes, report_bytes)`. The latter returns the fully recomputed `GuardReport` after exact byte-digest and binding comparisons. All bytes are supplied by the caller; the engine never fetches artifact URIs.

- `EngineBacked`: completed runs require all three engine references and later byte verification.
- `NativeOnly`: completed runs require domain evidence and null engine references; cannot satisfy engine-backed consumers. Complete native qualification is an external obligation, not inferred by this parser.
- Error/cancelled bound attempts have null decision, no report reference and diagnostics. Pre-binding failures use a separate `TransportDiagnostic`; it has no candidate, envelope or technical decision.

The JSON Schema closes every object, requires nullable keys to be present and checks basic field/profile shapes. Use a Draft 2020-12 validator. `$defs.engineBackedCompleted` and `$defs.nativeOnlyCompleted` can additionally constrain completed records; neither is a standalone envelope schema. JSON Schema alone is insufficient: the Rust loader additionally checks UTF-8 byte limits, sorted sets, coverage subtraction, chronological UTC timestamps, total size and nesting. Duplicate JSON keys must be rejected before a generic JSON object parser discards them. Rust deserializes directly into closed typed structs to reject duplicates. Direct Serde callers must still call `validate(profile)`; deserialization by itself is not eligibility or artifact verification.

Golden canonical bytes are compact typed-struct serialization, pinned by `tests/fixtures/integration-envelope/native.canonical` and its SHA-256 file. This is not a cross-language canonical JSON standard. Arrays representing sets must already be sorted; artifact/diagnostic ordering is preserved. Existing engine digests are unchanged.

Run:

```sh
cargo test --all-targets
python3 tests/schema/test_envelope_schema.py  # requires jsonschema, Draft 2020-12 support
```

The fixtures contain synthetic local binding labels, never authenticated production identities. A valid schema or matching digest cannot authorize an action. Trusted issuer, Git object resolution, baseline approval, current policy and expiry/revocation enforcement belong to separate controller/eligibility integration. See [decisions](decisions.md).
