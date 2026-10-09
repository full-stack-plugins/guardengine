# Actual CodeGuard Ruff F401 compatibility extension

This adds a narrowly qualified **SDK** profile to the [original six-consumer matrix](local-consumer-matrix.md). It does not replace the older unqualified CodeGuard vectors, alter native CLI exits, qualify every language/category, or change the original pinned manifest. [Exact version pairs and raw hashes](../schemas/integration/consumer-matrix-ruff-f401.json) identify CodeGuard689a316cf87464d9ec267656e0a673a9516b45b0 and the independently extracted GEc80ec32 archive. The current GE consumer runs the same vectors; no N/N-1 promise is inferred.

The producer executed nine real local `codeguard lint python ... --file app.py --ruff-tool ... --format=json` cases with official Ruff0.16.8. Each was projected under separately frozen enforce and review contracts, producing18 artifact sets. Native feedback is retained byte-for-byte and digest-checked as a domain attachment. Candidate/base IDs and timestamps in these SDK fixtures are synthetic; real source/tool/command observations are not Git binding or authenticated producer proof.

|Actual case|Direct Ruff outcome|Original CodeGuard outcome|Narrow SDK outcome|
|---|---|---|---|
|F401 violation|1|3, unchanged|complete; enforce BLOCK / review REQUIRE_APPROVAL|
|Fixed / clean file|0|3, unchanged|complete; ALLOW for the single frozen F401 obligation|
|Rule disabled / noqa / per-file ignore / changed source|0 in the captured controls|3, unchanged|partial; BLOCK even under review|
|Tool error|2|3, unchanged|partial; BLOCK|
|Missing tool|spawn failure, no fabricated exit|3, unchanged|partial; BLOCK|

The common technical decision comes from the supported narrow observation and protected mapping. Numeric native3 is never blindly converted into approval. A narrow ALLOW does not upgrade the aggregate native command; its original3 remains in the attached bytes. Caller-selected versions, broader contracts, unresolved native coverage and unsupported source/configuration combinations remain subject to the CodeGuard reader's rejection/partial rules. Ordinary `guard-project` remains an unqualified run_report1.0 interface at this producer pin; no new CLI0/3 path is claimed here.

`cargo test --locked --offline --test consumer_corpus` verifies both manifests:34 cases total, including the original explicitly derived native-only vector. The new18cases check exact hashes, full engine recomputation, closed schema rejection, mutations of each core artifact, decision mismatch, stronger-capability denial, attached native bytes and command-aware outcomes. Independently extracted GE also verified18positive cases plus90mutations outside sibling checkouts. Domain qualification itself was separately reviewed in CodeGuard, not reimplemented in the generic engine.

Preserved-native evidence additionally includes the independently reviewed PR31 transplant, unchanged native modules/rulepacks, actual aggregate check parity, and original format/check-language/help regression targets. Source preservation and installed-tool qualification are distinct: all57 formatter routes remain, while absent native tools and untested platforms remain explicitly unverified. Public release, authenticated controllers and hosted enforcement remain outside this local compatibility evidence.
