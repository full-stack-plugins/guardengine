# Integration verifier YAML profile

`integration::verify_engine_artifacts` accepts a conservative YAML contract profile.
The native `load_contract_yaml`, `evaluate`, and existing CLI retain their behavior.
The typed `integration::evaluate_bounded` helper receives already constructed values;
it does not parse YAML or protect allocations made by its caller.

After exact artifact-byte digest validation and before calling the legacy contract
loader, the verifier walks `yaml-rust2` parser events without constructing a YAML
value tree or resolving aliases. It stops on the first unsupported construct or
budget violation. The pinned parser is `yaml-rust2` 0.10.4, with optional encoding
support disabled; its declared minimum Rust version is 1.65.

The integration profile requires UTF-8 and exactly one YAML document. JSON-shaped
YAML, ordinary mappings and sequences, plain scalars, quoted scalars, literal and
folded block scalars, and comments remain supported subject to native contract
schema validation. Anchors, aliases, and explicit tags are unsupported, including
unused anchors and standard explicit tags such as `!!str`. Literal `*` and `&`
characters inside scalar text are not rejected. Syntax must be accepted by both
the preflight parser and the legacy loader; this is intentionally a subset of the
legacy loader's input language.

| Limit | Maximum |
| --- | ---: |
| Raw contract artifact bytes | 16,777,216 (existing artifact cap) |
| Nested mapping/sequence containers | 64 |
| Parser events, including stream/document/container events | 65,536 |
| Decoded UTF-8 bytes in one scalar, including keys | 262,144 |
| Sum of decoded UTF-8 scalar bytes, including keys | 8,388,608 |

Event limits are enforced as events are pulled, before legacy deserialization.
The streaming parser must scan a scalar to emit its event, so an individual scan
can temporarily allocate up to the already bounded raw input size before the
scalar limit is checked. These are input/work bounds, not a claim of an exact
process RSS cap. No expanded alias data is ever supplied to the legacy loader.
The parser and scanner retain their normal bounded-by-input working state.
Subsequent typed evaluation budgets remain separate safeguards.

The regression test reproduces the independent review's exact 206,573-byte
contract: 500 rule descriptions alias a 131,072-byte scalar. The verifier rejects
its first anchor with `YAML anchors and aliases are unsupported`; the native loader
still accepts anchors. Tests also cover event/depth/scalar budgets and acceptance
of stars/ampersands in quoted, plain, literal and folded text. No test substitutes
a synthetic allocation count for a real memory measurement.
