# Actor handler selector and remaining health boundary

2026-10-05; **INVESTIGATING / PARTIAL**. Reviewed clean67ba713, state,
architecture, handoff14, native14/source01 and Rust actor tests. Used BO2
reverse-engineering skill. Read-only investigation; no gameplay/UI/health
sampling, writes, injection, suspension or callback-graph survey.

Observer08 process42012/creation134357079689703421 and executable SHA-256
6c405bec03a89b8d954d8ae5edea1bed89b4147f53e4dbe68ced6ef5a815a96b
requalify. Parent reports fresh Origins solo, Nikolai500, paused at spawn,
Plutonium r5354. Steam script build24784239 equivalence remains distinct.
External task15 inspector requires named record, image/header/native profile,
executable pages and repeated identical code. Original output stays external.

First lead `getactorarray` has no candidate in admitted registration regions;
absence is bounded, not universal. Source01 actually calls `getaiarray`.
This named ID0x1c7, args0/10, recordRVA0x86be20, functionRVA0x440f90
provides the second, successful locator lead. No third lead or value scan needed.

Native first/next helpers iterate at most32 slots, stride0x2720, from pointer
at0x23387ac. Active-bit/team-mask admission derives from those helpers, not
guessed health/type values. One selected active-mask slot3 has entity index25,
team2. Entity table/stride matches native14; actor+4 points to that entity and
entity+0x158 points back to the same actor. Identity/selector/pointer rereads agree.
Entity+0x172 selector2 resolves exactly one death-handler slot:
RVA0x94224c -> functionRVA0x1df00. This proves an active AI-backed linkage;
team2/selector2 alone does **not** prove ordinary Origins zombie archetype.
The full getaiarray exclusion-filter helper is also untraced; no full enumeration
or actor-classification claim follows.

| Qualified complete span | Bytes / SHA-256 |
| --- | --- |
| getaiarray RVA0x440f90 |121 /995779c393e80937678d694e91be128746bd74b0f45555a0efb48c0312ab3fc8 |
| first RVA0x2ce760 |68 /e888d78708db76234c955b9be356cf4b4ad27a7cd22015d525eb999221fb6ecd |
| next RVA0x215930 |101 /a54ecd75ce4ff5b92a0bb5d2cb6940a6c421adf8f11a2c180a00772682374a91 |
| selected handler RVA0x1df00 |504 /04c7a0eb7292afe2ac13f69c3f6e0a5b5c021d77a6b861ff45d0ec824ce484ff |
| callback wrapper RVA0x2f2b00 |286 /f1eaaf3c0f8cafdaeba70b120f10ebe4c4f8e7f6d5e6aa7aa72420ccc752697e |

Selected handler checks actor state against6, calls the callback wrapper, passes6
to state helperRVA0x150c0, and directly clears actor+0x1bc8. This clear supports
native14's repeat-impact rejection gate. It also updates flags/metadata and calls
cleanup helpers. No direct entity+0x1a8 store is present in this handler, but
opaque helpers prevent inferring no final health change.

The wrapper passes eight arguments, entity and runtime handle at0x23f940c to
dispatcherRVA0x37f90, consistent with source actor-killed callback routing.
Dispatcher/script and state/cleanup effects remain untraced. Static selection
is not proof that this handler executed during a lethal impact.

**Recommendation:** preserve ActorDead zero normalization as explicitly
APPROXIMATE; neither final zero nor retained signed terminal health is proven.
Precise next edges are ordinary archetype admission and the selected handler's
callback/state/cleanup health effects. Do not replace terminal storage from
absence of a direct store. A controlled admitted ordinary-actor before/after
measurement could discriminate final health without widening the graph.

Actual reads12,207,741 bytes including profile checks<24MiB; retained binary
spans5176bytes<12KiB. Named parser five cases and pointer valid/truncated/null/
misaligned cases pass. Executable/code/record and actor backlink rereads pass;
all successful inspections exit0. External receipts preserve discovery/boundary
spans. Content/diff checks pass; Cargo unchanged/not rerun. No fidelity promotion.
