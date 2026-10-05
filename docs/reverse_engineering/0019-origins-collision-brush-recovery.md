# One Origins collision brush recovered

Task0028, 2026-10-05. Data access is **INVESTIGATING**; original runtime
collision and gameplay fidelity remain **UNKNOWN**. This resolves the root
admission failure in [findings0018](0018-origins-collision-brush-boundary.md)
and acquires one inspectable brush. It does not establish a collision API or
original contact tolerances.

## Sources and bounded procedure

The only original input was `zone/all/zm_tomb.ff`, 78,753,344 bytes, SHA256
`663c2f6e275fb6a8a2844bcc0e31808abe681ce7c2a2ddc6f616ef98fc7cd0b5`;
selected family was `maps/mp/zm_tomb.d3dbsp`. Real paths are excluded here.
The retained OAT revision remains `7d027e8f89118196713e955b0e11f8404149c54d`.
Relevant source was the existing T6 declarations, clipmap loading rules and
generated `clipmap_t_t6_load_db.cpp`, plus the task0026 production adapter,
arithmetic checks, actual fixture, observer, supervisor and analyzer.

Generated loader lines659-682 loads embedded ClipInfo, then conditionally loads
or resolves a separate `pInfo` through allocation/alias handling. Lines333-359
loads parallel bounds/contents only when their pointers are present. The rules
give those arrays counted/reusable relationships, without marking them runtime.
Neither source nor this observation establishes original runtime provider choice
or how absent arrays are derived by the game.

Both attempts used the normal signed, single-zone LIST route: no bypass,
preload, extraction or other original input. Existing supervision retained a
120-second observed deadline, 2 MiB combined capture cap and one external record
capped at64 KiB. Header diagnostics contain scalar counts, presence/alignment
booleans and named arithmetic predicates, never addresses or materials/scripts.
Before a separate header dereference, its72-byte span and alignment are checked.
Decoder allocation validity remains trusted; address arithmetic is not proof
of an allocation extent or a hard native memory/process-tree bound.

Each original attempt had same-handle64 KiB-block, 128 MiB-capped full identity
checks before and after. All four identities agree. The decoder separately
name-opens its input, so preservation is conditional on that name resolving the
checked input during decoding. The two-attempt allowance is exhausted.

## Exact gate and provider observations

Both reads observed the same embedded header:

| Array | Count | Observed presence / arithmetic admission |
| --- | ---: | --- |
| Planes | 12,105 | Present; pass |
| Sides | 17,108 | Present; pass |
| Vertices | 68,642 | Present; pass |
| Brushes | 8,417 | Present; pass |
| Parallel bounds | 8,417 | Absent |
| Parallel contents | 8,417 | Absent |

`pInfo` was null. Attempt1 rejects specifically at **bounds.nonnull**, with
contents.nonnull also failing, at stage3 before any candidate read. It exits123
in1.563 seconds, captures1,129 bytes and produces no brush record. This identifies
the old aggregate gate; it does not establish missing core geometry or corrupt
input.

Outer review admitted a second schema28 candidate that requires the four core
arrays and validates every present parallel array. Absent parallel arrays are
explicit presence=false/JSON-null fields, with comparison results unavailable.
There are no synthesized bounds or contents. Plane signs and analyzer tolerances
were unchanged between the original attempts.

Attempt2 exits0 in3.734 seconds with stages1-9, captures1,994 bytes and selects
the first eligible candidate, **brush index7**, after seven count rejections.
It has four sides and16 vertices. Side/vertex spans and every side-plane pointer
join to the admitted counted pools before body reads. Selection remains ascending
among the first32 candidates, with1-64 sides,0-256 vertices and finite positive
asymmetric endpoint extents. No selected contradiction was skipped.

The external record is1,656 bytes, SHA256
`0b3cab3d285b299712879cff25924100a93f6495f7f179f7ec4b1be8d0e7c17f`.
Original float32 bits, order and opaque brush/side metadata remain external.
All supervision limits pass; both children and capture threads are cleaned.

## Geometric discrimination and limits

The unchanged four-hypothesis analyzer visits480 plane triples. All16 stored
vertices lie exactly within the brush endpoints and their extrema match those
endpoints exactly. The four side-normal lengths are approximately
0.999999982885729. Original spatial units remain uncalibrated.

Only `normal dot point - distance <= 0`, combined with six endpoint-derived
axial constraints, reconstructs a full-rank16-vertex solid with exact endpoint
extrema. The other three hypotheses reconstruct no vertices. This is evidence
favoring that sign interpretation, not full compatibility: the favored candidate
has eight exact and tolerant source-constraint violations, and bidirectional
stored/reconstructed vertex-set disagreement of0.0005449587606563 in raw units.
Consequently **no hypothesis passes the predeclared full compatibility test**.
No tolerance was increased to obtain success. Quantization, stored-vertex purpose
and original collision epsilon remain hypotheses requiring measurement.

Independent parallel bounds and contents comparison is unavailable, not passing.
Runtime use of this brush, collision masks, player dimensions, basis, speed,
fixed timing, step/slide behavior and contact rules remain unmeasured. This is
one decoded primitive, not all-map collision, a player implementation or a live
original-game comparison.

## Validation and retained evidence

The exact compiled production adapter passes23 initial cases, including separate
provider, alias, negative count, each relevant null/count/range rejection and
pre-dereference malformed provider pointers. The schema28 change passes six
actual adapter cases: present, both absent, each single absence and malformed
present parallel arrays. Eight paired/presence/type consistency cases and three
missing explicit nullable-field cases reject; the asymmetric synthetic wedge
continues to select only the minus-distance, at-most-zero interpretation.
Incremental adapter compilation, production linking and actual fixture linking
all exit0 with native warnings treated as errors. Synthetic success is separate
from the original mismatch above.

External evidence uses relative identifiers `original-attempt1-result.json`,
`original-attempt2-result.json`, `original-attempt2-geometry-summary.json`,
`optional-adapter-source.diff`, `optional-analyzer-source.diff`, build/test
receipts and the associated private record. `attempt1-candidate/` retains the
first attempt's source/tools; the task0026 touched inputs were not modified.
There are no Rust/Cargo/architecture changes. Unchanged workspace gates are
intentionally skipped for this evidence-only task.

The next bounded task should resolve float32 storage/solver precision using the
retained brush before defining the safe owned Rust boundary. No additional decoder
read is needed for that question. Original runtime collision semantics still need
measurement before diagnostic tolerance can become gameplay behavior.
