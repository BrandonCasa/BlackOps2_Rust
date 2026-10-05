# Native player damage writer and signed admission

Recorded 2026-10-05. **INVESTIGATING**: inspected runtime code, not proof of live
execution. Clean e54ee80/state/architecture, source01, native04, observer08,
diagnostic09 and actual damage/API tests reviewed; reverse-engineering skill used.
Only query/read access; no UI, sampling, writes, injection or gameplay changes.

Existing process42012/creation FILETIME134357079689703421 and observer08 profile
requalify. Runtime r5354 identification is parent UI evidence; Steam script
build24784239 and loaded runtime-script equivalence remain distinct. External
task10 native-damage.py reads named registrations and two code spans, emits only
external raw code/disassembly and metadata. Discovery reads6,090,674bytes under
24MiB; code retention2736bytes under8KiB, each span at most4KiB. No value scan or
full-process dump. Native executable image pages, names, registration relationships
and repeated identical bytes pass.

| Runtime locator | Observation / SHA-256 |
| --- | --- |
| finishplayerdamage ID0x197, args11/11; entryRVA0x868b60, nameRVA0x842b2c, functionRVA0x429790 | Complete2557-byte function; a07bbe0244fe3276b3ebc5ef61cc8ad66c83e46cc8f99a5f4479494d96b982bd |
| Integer argument helper RVA0x9a060 | Complete179-byte function; 62b377779059f45c3a244ad65d7a4f10b29fbf9b21baca61170d0929b538c50c |

The method resolves ordinary script entity index to **0x21c13c0+index*0x31c**,
requires a client pointer at+0x154 and reads damage argument2 through that helper.
Accepted VM tag7 returns its32-bit payload unchanged. Other tags take type-error
handling; no float-to-integer conversion occurs in this accepted helper path.
Upstream script numeric conversion remains separate. Damage uses a signed
nonpositive rejection before proceeding; consequently u32::MAX's bit pattern
represents-1 and does not behave like Rust's admitted huge positive damage.

Under ordinary admitted positive damage, excluding native optional scaling/policy
branches, RVA0x429e69 directly subtracts resolved amount from **entity+0x1a8**.
This is exactly observer08's table/current locator, not a second candidate table.
Subtraction is32-bit machine arithmetic, without saturating/overflow handling at
that instruction. Positive current/damage both<=i32::MAX give an overflow-free
difference. Post-subtraction health is tested signed-positive; ordinary nonlethal
path later mirrors current to **client+0x238**, atRVA0x42a17c. Notifications and
indirect callbacks occur between these operations; their side effects/execution
are not exhaustively qualified here.

Raw lethal handling is layered: after intervening laststand/death callbacks, a
still-nonpositive health below-999 or exactly0 becomes-999; values-999..-1 remain.
This is not universal zero-clamping. Source01 ordinary coop laststand assigns1;
do not replace Rust Downed1 with this raw lethal sentinel. Actor arithmetic remains
unqualified. The resolved boundary excludes raw-impact scaling/callback policies.

**Recommended correction:** reject positive damage above i32::MAX instead of
accepting unrestricted u32. Native04 setmaxhealth uses this integer helper and
writes32-bit current/max; getnormalhealth interprets current/client-max through
signed integer-to-float conversion. A positive1..i32::MAX health/max admission is
therefore a justified safe project subset of native signed representation. The
native setter does not itself enforce positivity; do not claim original constructor
validation. Retain checked Rust arithmetic and evidence-backed downing/restoration.

**Missing active edge:** current registration maps ID0x197 to this function, but
the active VM builtin-method lookup/dispatch (including any Plutonium override)
was not located. Bounded registration-region pointer cross-reference finds only
record+16, RVA0x868b70; it does not establish an execution slot/call. Thus writer
correspondence is confirmed for this native implementation, not actual trial09
dispatch. Unchanged live100 remains unexplained. Next qualify that exact active
dispatch edge before another damage comparison; no broader survey performed.

External locator/profile/helper receipts retained. Actual registration parser
synthetic valid/truncated/name/argument-order/function-range cases pass; final
code spans/hash/page/repeat checks pass. Permitted-content/whitespace checks pass;
Cargo unchanged/not rerun. No fidelity promotion or execution observation.
