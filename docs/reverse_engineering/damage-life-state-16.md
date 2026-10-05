# Bounded ordinary-zombie identity investigation

2026-10-05; **INVESTIGATING / PARTIAL**. Reviewed clean0fdf560, state,
architecture, handoff15, source01/native14/15 and Rust actor tests. Parent's
later2a22eb5 documentation correction preserved. BO2 reverse-engineering skill
used. Two identity leads exhausted; no per-entity ordinary predicate proven.

**Source lead.** Shared `_zm_spawner::zombie_spawn_init` assigns targetname
zombie, clears script_noteworthy, sets is_zombie, then permits custom spawn
hooks. These generic assignments alone cannot exclude special actors. Origins
capture source registers a capture spawn hook; tomb's chamber capture hook sets
a capture clientfield after emerging. Mech source uses its own spawner family
through spawn_zombie. No proof follows from classname, team2 or selector2 alone.

Two additional licensed compiled buffers from source01's Origins patch were
decoded externally with pinned gsc-tool1.4.10.333, exit0:

| Zone-relative script | Bytes / SHA-256 |
| --- | --- |
| maps/mp/zombies/_zm_ai_mechz.gsc |39293 /558424a983b63d4c4e3065cac365265681290e77f60ae6bcae0cc1a3aa7e1e5c |
| maps/mp/zm_tomb_capture_zones.gsc |52210 /c32043a75c53eb5cae4779d2495e4b55500dfb5917e2743d2d9228305244857b |

Tool hash agrees with source01. Inputs read-only; decompiled source remains
external. Steam script build24784239 differs from runtime provenance. Vanilla
mech tracker waits until round8 and a zone activation; its FFOTD hook/runtime
equivalence remains unqualified. Fresh round1 at spawn with no objective/zone
activation therefore narrows special-spawn likelihood as a **controlled inference**,
not proof of the selected actor's archetype or every special exclusion.

**Native getter lead.** Qualified process42012/creation134357079689703421,
image/header/native profile from observer08 requalifies. Parent reports original
r5354 still paused; worker sends no UI input. Named getentarray ID0x233 has two
registrations. CandidateRVA0x178c80 demonstrably uses server entity table
0x21c13c0/stride0x31c; the other uses a client path. Complete329-byte server
routine hash4c91b6ef78dfd0c9dfb68bec05a243d8ce091cb70f3626700bbf371e242f1639
is derived from its repeated, executable-image-qualified capture.

Server getter calls field lookupRVA0x235c30, then indexes20-byte descriptors
atRVA0x86ccb0. For type5 it compares a16-bit entity field against a requested
string token. Bounded named descriptors establish classname+0x176,
targetname+0x17a and script_noteworthy+0x17c. They are token fields, not direct
character strings. Field-name pointers and only these descriptors were emitted
externally; no entity identity/health value scan or whole table dump.

Exact gap: selected entity's token-to-string qualification and special script/
clientfield exclusions remain unresolved. Even targetname zombie plus empty
noteworthy would require proving custom-hook exclusions. No guessed string-pool
address or VM-wide survey; selected entity25 remains AI-backed, not conclusively
ordinary. Keep ActorDead0 approximate pending final-health evidence.

Read totals6,102,091bytes including profile checks<24MiB; retained native
code1225bytes<12KiB. Named parser cases and synthetic descriptor valid/truncated/
name-range/alignment/entity-bound/type checks pass. Runtime page/hash/repeat and
named-field reads pass, exit0. One initial command failed before execution because
its external directory did not yet exist; corrected. Content/diff checks pass;
Cargo unchanged/not rerun. No writes/injection/suspension or gameplay edits.

Next: finite selected-actor before/after health comparison under controlled fresh
round1/no-objective conditions, explicitly recording classification limits and
visual association. Avoid another string-pool/VM investigation by default.
