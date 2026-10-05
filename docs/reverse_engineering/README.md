# Reverse-engineering findings

A bounded [local asset catalog](0001-local-asset-catalog.md) records a
Steam-registered BO2 source, aggregate file metadata, Origins candidate
identifiers, and representative prefix/hash observations. Data availability is
INVESTIGATING. No scripts, entity definitions or live behavior have been inspected;
gameplay and visual fidelity remain UNKNOWN. No engine/game API is approved from
the filename and prefix evidence.

Each bounded investigation should record its question, source/build identifiers,
reproducible inspection procedure, observed structure, permitted measurements,
evidence limits, inference labels, reusable-abstraction candidates, and unresolved
questions. Link supporting findings from implementation tasks.

Keep assets, copied scripts, copyrighted excerpts, raw extractions, dumps, and
installation paths external. Commit only abstract schemas, behavioral and
architectural descriptions, non-copyrightable measurements, appropriate hashes
or identifiers, and user-created or synthetic fixtures.

The [signed PC fastfile envelope](0002-fastfile-envelope.md) records six bounded
sample observations and primary-source corroboration of PC version 147, a
312-byte envelope and a conservative metadata-inspector contract. Format
understanding is INVESTIGATING; signature and payload validity remain unknown.

The [envelope inspector handoff](../handoffs/0006-fastfile-envelope-inspector.md)
records the implemented conservative metadata parser/tool, synthetic tests and
six unchanged external sample identities. Payload decoding, loader APIs, shared
game structures and gameplay require later evidence tasks.

The [signed PC transport framing](0003-fastfile-block-framing.md) records repeated
metadata-only scans through all six files, plaintext chunk lengths, zero suffix
accounting and the distinction between on-disk transport and internal authentication
state. It supplies an optional bounded scanner contract and recommends a single
external OAT asset-directory listing as the next access task. No decoded contents
or authenticity are established; format understanding remains INVESTIGATING.

The [decoded Origins candidate directory](0004-origins-asset-directory.md) records
two identical listings of one external `zm_tomb.ff` using the verified OAT release.
Its 7,549 entries across 31 types include map-world and geometry families. Counts
and reference markers establish directory availability, not dependency resolution,
decoded geometry properties or fidelity. It supplied the bounded external
metadata-adapter question investigated in findings 0005 below.

The [GfxWorld surface boundary](0005-gfxworld-surface-boundary.md) records two
identical aggregate observations through a separately instrumented pinned external
decoder. Its 6,469 surfaces partition all 449,721 index elements under the tested
triangle-range interpretation, and all material pointers join to 280 unique
non-reference-marked pool entries. The vertex slice candidate is not universal:
6,236 surfaces declare zero vertices despite nonzero triangles, and complete byte
spans remain unknown. These observations constrain the next per-surface vertex
stream investigation; they establish neither render readiness nor visual fidelity.

The [GfxWorld vertex-stream boundary](0006-gfxworld-vertex-stream-boundary.md)
tests a public Black Ops II extractor's 36-byte stream-0 addressing candidate
against all 6,469 surfaces, including every zero-count case. The 320 offset groups
account for all 191,259 logical vertex extents and the entire stream-0 capacity
with 16-byte alignment padding. Group logical-base intervals also partition the
logical capacity, while zero-only versus positive-only groups reject a simple
count-owner sentinel. The twelve earlier stream-1 failures are exactly end-offset
cases with positive counts; required stream-1 spans and attribute encoding remain
unknown. Structure remains INVESTIGATING; position-prefix validation is the next
bounded measurement before geometry import.

The [GfxWorld candidate position prefix](0007-gfxworld-position-prefix.md)
tests all 191,259 candidate records and all 449,721 per-surface references as
finite little-endian float32 triples. Every reference fits its surface's separate
`bounds[0]`/`bounds[1]` ordered-endpoint candidate exactly across all seven formats
and every zero-count surface. The embedded `tris.mins`/`tris.maxs` endpoint
candidate fails on every surface, so the two bounds families are not interchangeable.
This supports the candidate prefix without a unit conversion or geometry import;
original bounds/draw semantics, required stream-1 spans and other attributes remain
unknown. Structure remains INVESTIGATING; gameplay and visual fidelity remain UNKNOWN.

The [Origins diagnostic geometry snapshot](0008-origins-diagnostic-geometry-snapshot.md)
connects that evidence to the accepted safe Rust v1 consumer. Two complete external
productions preserve all 191,259 records, 449,721 ordered references and 6,469
surfaces, including all 6,236 zero-count cases. Their 4,404,568 bytes and source
prefix/reference/metadata/bounds bits agree exactly; independent file accounting
and each Rust reader match complete counts/categories. Tooling and derivative
geometry remain external, task9 is restored exactly, and the excluded monitoring
race/repair is documented. Data understanding remains INVESTIGATING; visual,
gameplay and overall fidelity UNKNOWN. The next bounded task uses Rust Bevy Mesh;
full attributes/materials still need separate stream1/layout evidence.
