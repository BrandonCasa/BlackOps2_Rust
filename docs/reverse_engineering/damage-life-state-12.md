# Bounded finishplayerdamage dispatch investigation

2026-10-05; **INVESTIGATING / PARTIAL**. Reviewed clean9ea1515, current state,
architecture, handoff11 and [native10](damage-life-state-10.md). Used BO2
reverse-engineering skill. Question: does active VM method ID0x197 dispatch to
the inspected native function or an override? Three concrete leads failed to
establish an execution slot; investigation stops here.

External task12 dispatch.py uses query/read access only. It reuses observer08's
process creation identity134357079689703421, executable SHA-256
6c405bec03a89b8d954d8ae5edea1bed89b4147f53e4dbe68ced6ef5a815a96b,
header/code/name/registration checks and admitted executable image pages.
Parent UI identifies Plutonium r5354; Steam script build24784239 equivalence
remains unqualified. Process42012 was readable; no UI, suspension, sampling,
injection or writes occurred.

ID0x197 registration at RVA0x868b60 still names finishplayerdamage, args11/11,
function RVA0x429790. Its2557 bytes match native10 SHA-256
a07bbe0244fe3276b3ebc5ef61cc8ad66c83e46cc8f99a5f4479494d96b982bd.
This is a registration relationship, not proof of active dispatch/invocation.

1. Search admitted executable image for exact entry/name/function address
   immediates: no matches.
2. Bounded structural walk finds42 compatible24-byte records at
   RVA0x868ad0..0x868ec0, target index6. Executable references to run boundaries,
   first ID/function fields: none. This predicate identifies a compatible run,
   not a proven complete VM table or initializer.
3. Search the previously admitted registration-data regions for pointers to that
   run, target entry and function, then executable consumers of matching fields:
   only known target record+16 at RVA0x868b70; no executable consumer found.

Each executable search covers RVA0x1000, length7,332,677. All three captured
search-buffer hashes agree:
e205b25c0c801f22470f30d4d588f132b4eb1cf289b97ae13c7c05bae63bd0a7.
Reads per successful lead:7,339,905;7,340,937;13,421,976 bytes, each below
24MiB invocation budget. Full search buffers are not retained; external receipts
contain locators/hashes/counts. No whole-process dump. An initial name-length
qualification error stopped safely; corrected19-byte name check then passed.
Synthetic predicate cases valid/truncated/name-range/argument-order/function-range/
ID-range pass. Helper and receipts remain external; no new dependencies.

Absence of these immediate references does not exclude computed/indexed access,
other module dispatch, or copied/overridden tables. Exact missing edge is the
active VM method-ID lookup/load and resulting callable pointer for0x197.
No override classification or fidelity promotion is justified. Next separately
compare qualified client+0x238 mirror against entity current during controlled
damage, rather than repeat this bounded VM survey.
