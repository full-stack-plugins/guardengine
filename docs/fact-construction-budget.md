# Integration fact construction budget

`integration::FactBudget` is an opt-in borrowed-field projection builder. Call
`push_relation(subject, predicate, object, source)` with references before copying
native finding or contract strings, then `finish()` to obtain the owned facts.
Each insertion checks the serialized escaped JSON size plus owned string bytes
and conservative vector element overhead against a cumulative 16 MiB budget;
4096 facts is the count limit. This is a conservative admission estimate, not an
allocator or operating-system RSS limit. Any rejected insertion poisons the
builder, so callers cannot ignore the error and return a partial successful list.
The builder owns no domain mapping, approval or policy semantics.

Producers must still bound their native/domain inputs and expansion **before**
constructing assessments, missing-edge lists or formatted source strings.
This builder cannot reclaim allocations already made by callers. Use
`evaluate_bounded` after constructing the full GuardFacts object to bound report
fanout and serialized metadata. Neither surface changes legacy native evaluate.

Validation: `cargo test --locked --test integration_fact_budget` covers ordinary
field preservation, escaped encoding size, cumulative repeated-field expansion,
count limits and rejection poisoning. Separate domain tests must exercise the
caller's preconstruction boundary.
