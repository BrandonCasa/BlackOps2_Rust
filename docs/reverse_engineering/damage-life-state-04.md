# Native revive health restoration

Recorded 2026-10-05. Question: what current-health effect does native reviveplayer
have before the ordinary Origins script's max-health reboot? **Resolved for the
observed Plutonium process:** reviveplayer copies client maximum into entity
current/maximum; subsequent setmaxhealth writes its integer argument to both.
Therefore the task03 no-bonus health-reboot argument100 produces current100 and
maximum100 under that qualified native implementation. This is native code
inspection, not a completed live coop revive comparison. Fidelity INVESTIGATING.

## Provenance and source distinction

Licensed Steam Zombies app212910 build24784239 t6zm.exe: length12543488, SHA-256
f6f7104af2bd0c2b931ea1e739969e85845e499affbf686144c6b490b344ab1e;
identity unchanged after task. PE32 x86, preferred image base0x400000. On-disk
reviveplayer name at file offset0x740a8c yields registration ID0x566 and target
RVA0x1ee2c0; setmaxhealth ID0x667 targetRVA0x226c10. The correct static targets
do not provide coherent admitted function decoding. This supports a protection
hypothesis but does not establish its mechanism. No attempt to bypass it made.

Outer UI observation identified an existing Plutonium r5354 Origins lobby. Worker
read that same live process without UI/input: process creation UTC
2026-10-05T21:06:08.9703421Z, image basename plutonium-bootstrapper-win32.exe,
on-disk image length19151152, SHA-256
6c405bec03a89b8d954d8ae5edea1bed89b4147f53e4dbe68ced6ef5a815a96b.
Mapped PE32 timestamp1379794238 and header SHA-256
d203ed499e46f35abf308fd88c4069357a86b1d654d4a6b87b236cf2e152b161.
Runtime sections differ from Steam file: static RVAs are **not** transplanted.
Runtime patch effects and loaded script identity were not independently verified.
The r5354 label comes from outer UI observation, not worker-derived file version.

| Runtime registered method | ID | Entry RVA | Function RVA | Exact code bytes read | SHA-256 |
| --- | --- | --- | --- | ---: | --- |
| reviveplayer | 0x566 | 0x868b90 | 0x42a190 | 390 | 6950b8f09dd6f8906d45e2e6b9b3ae01b22ab066504b61979024648b87440075 |
| setmaxhealth | 0x667 | 0x944fc8 | 0x44a200 | 72 | f3c134dc48270fb973c6df9c9c71eb96bf9b9f53d7886064ff793f3a76354a2c |
| setnormalhealth (field corroboration) | 0x683 | 0x944fb0 | 0x44a110 | 225 | bc5a95e9b737ea5a35a6f9335135bdeaca761601ceba934cd895e92df65b456e |

Both reads of each span and registration entry match. VirtualQueryEx admits each
complete span as committed PAGE_EXECUTE_READ (0x20), MEM_IMAGE (0x1000000), inside
runtime .text. Exact names/IDs and surrounding named records establish registration
layout: name, ID, argument bounds, function pointer, reserved field (24bytes).
First exploratory neighboring-pointer hypothesis was rejected because it led to
the preceding method; adjacent finishmeleehit/getnormalhealth names and function
semantics discriminate the layout. Final receipt uses the function at entry+16.

## Procedure and read/emit boundaries

External task directory below configured inspection root: damage-life-state-04.
No original bytes/scripts, raw disassembly or real paths enter tracked files.
Local helpers/proprietary spans and receipts remain external. Capstone5.0.6's
cached1.3MB Windows wheel was installed only into this external task's python-deps;
no global install, full decompiler or runtime hook added.

1. Read/hash licensed executable and parse PE metadata. Locate exact builtin names
   and candidate pointer records. Correct static code is not admitted as executable
   function evidence; retain that distinction rather than invent disassembly.
2. Identify live process by name/creation time and mapped PE header. Open only
   PROCESS_QUERY_LIMITED_INFORMATION plus PROCESS_VM_READ; no write/injection rights.
3. Read mapped header and bounded runtime .rdata0x1cca0f bytes plus first
   .data0x400000 bytes in memory to locate exact terminated names/pointers. These
   temporary scans are not saved; selected locator metadata alone is retained.
4. Validate selected24-byte records, name identity, IDs/argument bounds and function
   pointer within .text. VirtualQueryEx checks committed executable span before
   selected code reads. Do not apply the old Steam RVAs to this image.
5. Read only selected390/72/225-byte code spans twice; verify unchanged. Decode x86
   with Capstone; complete byte consumption and function return boundaries checked.
   Preserve these small spans/disassembly externally for review, not full memory dump.
6. Record runtime-qualified-receipt.json and process-identity.json, including
   code/registration/header identities and page qualification. Rehash original.

OpenProcess/ReadProcessMemory are inspection only. No game launch, input, setting,
console/mod/cheat, pause, entity mutation or gameplay measurement performed.
The required desktop Plutonium route remains applicable to later UI comparisons.

## Abstract native contract and field corroboration

Runtime reviveplayer validates script entity representation and client pointer;
ordinary player path clears several native state fields and calls engine helpers.
The final straight-line block copies the client maximum field into entity
current and maximum, and mirrors current into a client state field. There is no
immediate numeric100 in that method: it restores the current configured maximum.
Unresolved helper/flag meanings are not claimed reconstructed.

Runtime setmaxhealth resolves the script entity, obtains its integer argument,
then unconditionally writes that argument to entity current/maximum. If a client
pointer exists, it writes client maximum and a client state field too. Therefore
task03's disabled script clamp does **not** mean current health remains untouched:
native setmaxhealth itself resets it. This counterexample corrects the earlier
source-only uncertainty without weakening current/max distinction in Rust.

Entity current/maximum field offsets0x1a8/0x1ac and client maximum0x54cc are
identified through the registered setter plus getnormalhealth's current/maximum
ratio and setnormalhealth's scaling from that maximum. These offsets are evidence
locators, not a Rust entity layout. No broad engine schema is approved.

The valid ordinary coop completion composition is:

- native reviveplayer restores its existing configured client maximum;
- shared health-reboot computes100 when persistent health bonus is absent;
- native setmaxhealth then sets current/maximum100;
- script removes downed trigger and clears ignore as in task03.

This supports an Origins completed-revive restoration rule100/100 for the admitted
no-perk/no-bonus ordinary human route using the observed runtime native behavior.
Keep source Steam build and runtime r5354 qualification explicit. Neither the
downed state nor stale-attempt rejection is enforced solely by native setter;
Rust must require the current downed episode before applying the resolved result.
Do not extend to actors, max-health bonuses, device/solo revive, or earlier/later
Plutonium builds. A later live comparison can disprove runtime/source composition.

## Validation and remaining comparison

External registration parser synthetic checks accept one valid record and reject
truncated length/invalid name-pointer/function bounds. Actual names, argument
bounds, executable pages, whole-span decode, repeat hashes and original hash
preservation pass. Initial same-RVA attempt failed name validation and admitted
no code; runtime rediscovery resolved it. Inspection did not execute the builtin.
No new tracked inspector or Rust change; Cargo gates not rerun for docs only.

Next comparison: through desktop Plutonium, record runtime version/loaded script
overrides and use ordinary coop completion. Observe positive current/max via a
separately qualified narrow read-only field observer immediately around revive,
or retain numeric limits if video only. These located native fields provide a
concrete observer lead; this task did not read any player's health or execute a
revive. Actual completed transition, cancellation and live/native agreement remain
unverified. No health result should be labelled MATCHED/VERIFIED from disassembly.
