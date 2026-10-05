# Ordinary actor native health arithmetic

2026-10-05; **INVESTIGATING / PARTIAL**. Reviewed clean3e2feb9, current state,
architecture, handoff13, source01 shared actor wrapper, native10 and actual Rust
actor/tests/demo. BO2 reverse-engineering skill used. Scope: named actor native
subtraction/nonpositive branch; no AI, special policy or active VM survey.

External task14 native-actor.py reuses existing query/read inspector and observer08
identity/profile. Process42012 creation134357079689703421, executable SHA-256
6c405bec03a89b8d954d8ae5edea1bed89b4147f53e4dbe68ced6ef5a815a96b
requalifies. Parent UI identifies Plutonium r5354; Steam script build24784239
loaded-runtime equivalence remains unqualified. Named record/name, executable
image page and repeated bytes pass. Originals/code/disassembly remain external.

| Locator | Qualified observation |
| --- | --- |
| finishactordamage ID0x196, args10/12 | recordRVA0x87d498; nameRVA0x83e840; functionRVA0x509b70 |
| Complete1060-byte function | SHA-256 fa1961732200e7dfc6dd10515788c0b2aafd3e53c23bbc506e5c9e697d1e4ea3 |
| Integer helperRVA0x9a060,179bytes | native10 hash62b377779059f45c3a244ad65d7a4f10b29fbf9b21baca61170d0929b538c50c agrees |

Source01 `_zm::actor_damage_override_wrapper` forwards integer resolved damage
unless cannot-die policy blocks lethal damage. Exclude that policy and Origins
special cases. Native resolves script entity index using table0x21c13c0/stride0x31c,
requires its actor pointer+0x158, then accesses the actor's entity pointer+4.
An additional actor+0x1bc8 gate must be nonzero; its full semantic admission is
not established. Argument2 uses the same tag7/raw32-bit integer helper as native10.
Signed damage<=0 exits; positive1..i32MAX project bounds remain appropriate.

AtRVA0x509e84, direct32-bit SUB writes resolved damage against entity+0x1a8.
Admitted positive current/damage bounds avoid arithmetic overflow. No zero clamp
occurs there. An intervening notification callRVA0x190a0 precedes health reread;
its side effects are not traced. Reread signed health>0 selects one handler;
health<=0 selects the other. Equality therefore enters the nonpositive branch
if intervening effects do not change health.

Nonpositive health below-999 is floored to-999 atRVA0x509ee6. Zero and-1..-999
remain unchanged at this stage. After another notification, entity typebyte+0x172
selects a48-byte-stride handler record: nonpositive handler pointer at
RVA0x9421ec+type*48; positive handler atRVA0x9421e4+type*48.
No native health write follows the handler call before this routine returns.
The nonpositive branch is the death-route inference consistent with source
actor-killed routing; final terminal predicate/effects depend on that handler.

**Rust consequence:** ordinary nonlethal subtraction is now statically supported.
Unconditional native lethal zero-clamping is contradicted before the handler,
but this does not yet disprove final callback-normalized zero. Existing ActorDead
health0 remains an explicit approximation, not native proof. Smallest next edge:
qualify the ordinary actor's handler type/pointer and its final health/death effect,
without a general callback graph, before prescribing signed terminal storage.
Do not change human Downed1 or claim active dispatch/execution from registration.

Final inspector run exit0:6,087,679 read bytes<24MiB; retained code1239bytes<12KiB,
only named routine/helper. Earlier1024/1152-byte boundary reads are external;
final1060-byte span ends at return/padding boundary. Existing five registration
parser synthetic cases pass. Content/diff checks pass; Cargo unchanged/not rerun.
No UI, live actor measurement, injection/writes, dependencies or gameplay edits.
