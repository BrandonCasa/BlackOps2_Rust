# Ordinary Origins baseline parameter admission

Task movement-native-04, 2026-10-05. PARTIAL, INVESTIGATING. No executable Origins
profile is approved yet. This separates static constants, script configuration,
weapon fields, their consumers and unresolved parameter initializers.

## Sources and reproduction

Native source remains the identity in [movement-native-contract.md](movement-native-contract.md).
Use ignored movement source locators and PE mapping; raw disassembly stays in
external `movement-native-04`. The retained Origins script configuration from
task01 remains admitted; no current script or dvar objects were sampled.

Weapon source is the accepted Steam24784239 `zone/all/zm_tomb.ff`, 78,753,344 bytes,
SHA256 `663c2f6e275fb6a8a2844bcc0e31808abe681ce7c2a2ddc6f616ef98fc7cd0b5`.
Pinned OATv0.33.0 weapon-only extraction reads that one fastfile and emits weapon
assets externally. The inspected `c96_zm` record is22,220 bytes, SHA256
`d8e3939de4baa69dcab7ab9e744913a0c926aa16371581c0a1347e560c430632`.
Only abstract field measurements enter this repository. Original data is not a
Rust fixture. OAT source WeaponFields binds these keys to WeaponFullDef.weapDef
float fields; its Win32 WeaponDef layout puts them at588/58c/590. The layout
calculation accounts for pointer4, bool1, uint16 alignment2, scalar/enum alignment4
and embedded vector sizes; it corroborates independently observed native reads.

## Admission table

| Input | Admitted source/consumer | Value or missing association |
| --- | --- | --- |
| Script movement scale | Origins personality/team callbacks -> native448990 -> client54e0 | 1 |
| Numerical movement scale | Initializer01be10 and player update16cc30 copy client54e0 -> state50c | 1 for the admitted script configuration |
| Player sprint duration | Origins callbacks -> native0e88d0 -> state4cc | 4000ms |
| Player sprint cooldown | Origins callbacks -> native0be2f0 -> state4d0 | 0ms |
| C96 ordinary movement modifier | BaseWeaponDef588 ->53d6c0 ->540f60 | 1 |
| C96 sprint-duration modifier | BaseWeaponDef590 ->08c440 | 1 |
| C96 ADS field | BaseWeaponDef58c ->0501b0 | Raw2; effective ADS behavior not admitted |
| Ground friction | Complete263de0 static float return | 5.5 |
| Ordinary standing acceleration | Walk544200 static branch | 9 |
| Stop speed | VA31fc4b0 ->298b90 ->53d9b0/53dc10 | Registration/type/range/default unresolved |
| Backward scale | VA31fe73c ->298b90 ->540f60 | Registration/type/range/default unresolved |
| Strafe scale | VA31fe83c ->298b90 ->540f60 | Registration/type/range/default unresolved |
| Slowdown curve inputs | VA31fe778/31fe67c ->298b90 ->04dfb0 | Registration/type/range/default unresolved |
| Base player speed | VA2338668 ->2909f0 ->034ad0 state94 | Default/type/range unresolved; perk branch modifier15cd80 |
| Gravity | VA31fc4d8 ->298b90 ->conversion ->16cc30 state8c | Default/type/range unresolved; explicit per-owner override supported |

These are source defaults/configuration for distinct admitted builds, not sampled
active values and not a cross-build fidelity comparison. No numeric stop-speed,
base-speed, gravity, backward/strafe or slowdown default is inferred from names.

## Weapon definition and ordinary modifier ownership

Complete native1906b0 (20 bytes) masks the weapon identifier to8 bits, indexes
global VA321cbf8 and returns the entry's pointer at+8. OAT WeaponVariantDef names
its third Win32 member at+8 as the baseWeaponDef pointer. OAT's weapon field table
maps the three measured values to that base definition. Thus this is the base
definition consumed by540f60, rather than a coincidentally matching variant offset.
The Origins callback selecting `c96_zm` is qualified in task01. Runtime identifier
assignment and population of VA321cbf8 are still unresolved; no table slot number
is guessed for C96.

Complete53d6c0 (109 bytes) returns baseDef588 in the ordinary branch. A separate
perk/feature branch can clamp that value; this is excluded from the unperked
baseline. Complete08c440 (145 bytes) multiplies converted player duration4cc by
baseDef590 with SSE f32, optionally applies a perk multiplier, then truncates and
caps the integer at16383. Consequently the ordinary C96 multiplier leaves the
qualified4000ms player duration unchanged. Debit/recharge timing remains a
separate locomotion-mode task.

Complete0501b0 (127 bytes) reads58c and may replace it using attachment inputs.
540f60 chooses that consumer under predicates distinct from ordinary move-speed
handling. The numeric2 is configuration evidence only; it does not establish
that aiming doubles world movement speed, any interpolation or a host ADS binding.

## State initialization and indirect-store investigation

Complete01be10 (491 bytes) computes the client pointer using stride57f8 and
global VA2338780, associates entity+154 with it, initializes client54e0 to1 and
copies that exact value into numerical state50c. Complete16cc30 (2199 bytes)
obtains the same state through entity+154 and copies client54e0 to50c. This
establishes the script-to-numerical scale association; a nearby field name alone
would not establish it.

16cc30 assigns gravity state8c either from owner-associated configuration
(owner index -> stridec70, flag+c60/value+c64), or from VA31fc4d8 via298b90 and
native integer conversion. 034ad0 can also replace gravity from that explicit
owner configuration. It assigns base speed94 from VA2338668 via2909f0 and may
multiply it in a separately gated perk branch. Default gravity and base speed
remain missing despite these qualified ownership chains.

The initializer investigation included raw pointer-byte searches, decoded
consumers, pointer-base candidates near31fc/31fe, and bounded displacement-writer
searches for50c/54e0 followed by complete player bodies. It did not stop at direct
stores. Two immediate pointer-base candidates were excluded: one returns a
different object at31fc130, the other operates on a text buffer at31fe950. Other
50c writer hits belonged to unrelated receivers and were excluded.

No initializer/default association for the named dvar slots was recovered.
Recognizable strings for backward/strafe/stop speed had no direct pointer-byte
association in this disk image; names cannot resolve registration ownership.
The exact missing evidence is a registration/initializer mapping into those
global slots (including any indirect slot table or restored startup code), with
default/type/range and any Origins override. This investigation does **not** prove
that the mapping is in an absent DLL. Encoded startup dispatcher162b70 is retained
as a lead, but its association to the dvar slots is unqualified; no absent dynamic
module claim is substituted for the missing mapping.

## Arithmetic and validation limits

298b90 reads current dvar type/value;2909f0 has integer, bool, float-truncation and
text-conversion branches. Consumer type branches do not establish the registered
parameter type. x87 precision/control-word initialization and numerical integer
conversion helpers remain to qualify before implementing those arithmetic chains.
SSE f32 coefficient kernels and explicit integer command timing can be implemented
independently with explicit supplied inputs; this evidence does not authorize
inventing an executable Origins profile or generic x87 precision.

External receipts retain body hashes, source identity, field-layout calculations,
raw locator searches and repeat weapon extraction. No original process, play,
capture or execution occurred. A full-detail broad decoder scan was stopped for
resource use and replaced by bounded byte/displacement candidates with body
qualification; it is not counted as a complete scan. Documentation validation
passed; Rust/host/workspace gates belong to primary integration.
