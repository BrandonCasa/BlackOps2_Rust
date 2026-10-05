# Read-only local-player health observer

Recorded 2026-10-05. Observer qualification, **INVESTIGATING**, not damage fidelity.
One existing Plutonium r5354 process and one parent-controlled paused solo Origins
match admitted. No worker UI, console, memory writes, injection, scripts or mods.
Solo numeric samples cannot establish coop downing/revival/bleedout.

## Provenance and bounded route

Read [native restoration](damage-life-state-04.md), [bleedout](damage-life-state-06.md)
and [accepted implementation](../handoffs/damage-life-state-07.md). Current/max fields
are established by registered native getters/setters; local identity is newly
established below, not assumed from index0 or searched health values100/1.
Steam compiled-script build24784239 remains distinct from runtime native evidence.

Revalidated process42012, created2026-10-05T21:06:08.9703421Z
(Windows FILETIME134357079689703421), image basename plutonium-bootstrapper-win32.exe,
file SHA-2566c405bec03a89b8d954d8ae5edea1bed89b4147f53e4dbe68ced6ef5a815a96b.
Mapped PE base0x400000/header SHA-256
d203ed499e46f35abf308fd88c4069357a86b1d654d4a6b87b236cf2e152b161.
Parent supplies r5354 UI qualification. Original t6zm.exe/selected fastfile hashes
still match task04/task06; licensed inputs unchanged.

External task08 native-methods.py reuses cached task04 Capstone and query/read-only
OpenProcess0x1010. It reads .rdata plus first4MiB .data solely to locate exact names
and registration pointers, emits selected metadata/small code spans only, and
discards scan buffers. Discovery total6,086,201 target bytes under24MiB cap; no full
process dump or general schema. Final selected functions include complete return
paths; an initial larger exploratory span is narrowed in final receipt.

| Native locator, RVA | Observed relation | Final span SHA-256 |
| --- | --- | --- |
| islocalplayer 0x3c7390,40bytes | Calls local-index matcher; reports true when matcher does not return-1. RegistryRVA0x863170, methodID0x3a3, args0/0. | 3d96ff65eb8438815d7885590d019622d12be6c0012192fc527e32fc9790a205 |
| Local matcher 0x2972f0,68bytes | Admits one local-client slot; checks native active gate, follows global root then state pointer, checks state flags, compares entity index with state index. | f5e3286fe06cc0b5afe0270f2081edec16b905150616fa6a81c93e1c007b1d21 |
| Active gate 0x260c80,20bytes | Local slot0 tests bit0 of global activation word. | 88c94df8d17ac2642c2a59806dd7482a2a9fb32014c7c91761db6302c82b47b8 |
| Server entity resolver 0x2de010,62bytes | Valid ordinary script entity class uses low16-bit index, static table base and stride0x31c. Other class errors. | 2b1ccea9c7f2cf6e18bbdac1ceb89f633bedcc9901602f57024f2960f55e5faa |
| Server getentitynumber 0x44b820,29bytes | Uses that resolver and returns first entity word. RegistryRVA0x9459a0, methodID0x235, args0/0. Separate client-context registration of same name exists; it is not substituted. | 1eba983031fe787bfbdb27bb7f4c8e7cc103170a24a12e740d4de9f5020b049e |
| getnormalhealth 0x44a090,123bytes | Uses server resolver, entity current and client configured maximum for normalized player health; no-client route differs. RegistryRVA0x944f98, methodID0x291, args0/0. | 4aef8dbb166e0752ce30352d3bdb05e5180b591f79fd7cae8be3a199d8c98698 |

Concrete runtime locators (VA, evidence offsets rather than approved Rust layout):
activation word0x119c650; root-pointer word0x1113f9c; root+0x28 points to local state;
state+0x28 must contain flags masked6; state+0x1b0 gives signed local entity index.
Server entity is0x21c13c0+index*0x31c. Its first word must equal that index;
client pointer is entity+0x154, current entity+0x1a8, entity maximum+0x1ac,
client configured maximum+0x54cc. Task04 setmaxhealth's72-byte pinned function
corroborates both maximum fields; setter hash is retained in observer profile.
No activation flag is interpreted as proof of Standing, alive or a downed episode.

## External sampler and admission

All helpers/profile/receipts are external under configured inspection root
`damage-life-state-08/`. health-sample.py uses Python standard library only.
observer-profile.json pins PID/creation/image-file hash, mapped header, seven native
code spans and three exact registration/name records. qualify-profile.py is a
reviewed investigation artifact, not an automatic requalification/fallback command.
A restarted/changed runtime requires new explicit provenance review.

The sampler opens query/read only, checks profile before and after a finite run,
checks executable-image native pages and all derived readable committed spans,
refuses null/misaligned/out-of-range pointers and mismatched entity index, and
reads two identical snapshots before accepting a numeric sample. Each sample
checks process creation identity again. No cached pointer or previous health is
substituted. Dynamic indices are resolved afresh rather than table-scanned.
Read limits:4KiB per read,2MiB total per invocation,1–1000 samples, interval0–1second.
Observed accepted snapshot pair costs912bytes; qualification overhead4,628bytes
at run start and again at end. Disk image hashing occurs at qualification, not
per sample. The count400/interval0.05 run admits roughly20–23seconds including
acquisition overhead; the interval is a sampling choice, not game timing.

From external task08 directory run `python health-sample.py --self-test`, then
`python health-sample.py --pid 42012 --count 400 --interval 0.05` using the existing
qualified process/profile. Parent has the exact configured Python invocation.
Keep JSONL output external. Accepted output contains only PID/creation, local slot,
resolved entity index, signed integer current/entity maximum/client maximum,
monotonic acquisition-before/after nanoseconds and cumulative read count. Preserve
both maximums independently; do not assume they agree or clamp observed values to
Rust's admitted health domain. No player name, pointer dump or original bytes emitted.

Unstable/inactive/unreadable derived-state samples emit refused_sample plus timing
brackets **without health**, then continue; they never repeat prior health.
Unstable snapshots additionally report changed field names only, never pointer
values or raw memory. All identity/health checks remain unchanged. Full flag words
are conservatively compared even though native admission tests only bit0/mask6;
unrelated flag changes could cause refusals and require diagnostic attribution.
Process/profile/code/registration/header mismatch or budget exhaustion is fatal
(exit2); successful finite run exits0. Exit0 alone is not a valid sample count or
comparison pass: consumers must count accepted/refused records. No retry framework,
automatic runtime discovery or automatic regeneration of profile is added.

Double snapshots are conservative detection, not an atomic VM frame or entity
generation proof. A change-and-return between reads can escape detection; native
life/respawn generations are not reconstructed. Code/profile are checked at run
boundaries rather than every sample. Acquisition brackets bound read time, not the
exact hit time. Failures/gaps must remain gaps when plotting/comparing.

## Actual qualification and next comparison

Parent reports fresh r5354 LAN Origins solo, paused through Escape after spawn:
round1 Richtofen, score500, ammo8/32, no purchases/combat input and no console or
timescale changes in this match. Worker independently follows native gates/index
and observes5 retained repeated samples: local entity0, current100, entity max100,
client max100. Acquisition windows4.5141/4.9383/4.8172/4.8631/5.8703ms. This is a
numeric baseline plus observer provenance, not evidence for damage60, regeneration
rate, max100 universally, physical Standing, native terminal health or coop behavior.
Additional preliminary samples agreed; retained5 define the qualification receipt.

Parent's subsequent trial01 produced1000 records over55.743646seconds:966 accepted,
all100/100/100, and34 generic unstable refusals between approximately21.145 and
55.354seconds. Parent separately observed a solo result screen with one down.
Those generic records cannot recover which fields changed; no numeric damage
response or observer sensitivity was established by that trial. Afterward30
worker diagnostic samples refused an invalid local-state pointer, emitting no
health. Updated field-name-only diagnostics allow the next parent-controlled fresh
run to distinguish volatile flags from identity/health changes without relaxing
admission. Do not treat exit0 or constant accepted100 as a comparison pass.

For outer damage comparison: start finite sampler while paused; parent resumes,
admits one isolated ordinary round1 zombie hit, pauses promptly and records visible
hit/input/video timing. Compare stable pre/post current values and both maximums,
accounting for refused samples, regen before pause and any second hit. Repeat fresh
controlled trials; baseline100 to post40 would support resolved damage60 under
those exact conditions only. Do not infer damage merely from health reaching0,
assume UI and sampler clocks align, or use solo for coop bleedout. Damage/timing
results belong in the parent's separate comparison record, not this qualification.

Validation:15 synthetic cases pass, covering malformed/stale pointer/index/length,
snapshot mutation, stale process/header/code/registration/name and refusal followed
by fresh sample without stale health. Initial refusal-emission fixture needed its
mutation counter reset; repaired and final checks pass. Live native code/registry
identity/readable/executable pages and repeated paused samples pass. External
identity/locator/profile/synthetic/sample receipts preserved; no new dependencies.
Owned tracked findings/handoff content and whitespace reviewed; Cargo unchanged,
not rerun. No worker damage trial/UI or broad engine/memory implementation.
