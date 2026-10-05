# Standing-player trace request investigation

Task0037, 2026-10-05. **PARTIAL**; native request provenance is
**INVESTIGATING**, standing-player/gameplay fidelity **UNKNOWN**. This task
qualifies two nonzero-bounds request forwarding routes, but does not establish
ordinary standing-player dimensions, mask or stance selection. No player default
is admitted for the next Rust increment.

## Source and procedure

The read-only profile freshly qualifies process42012, creation
FILETIME134357079689703421, executable SHA-256
`6c405bec03a89b8d954d8ae5edea1bed89b4147f53e4dbe68ced6ef5a815a96b`,
image-header SHA-256
`d203ed499e46f35abf308fd88c4069357a86b1d654d4a6b87b236cf2e152b161`
and the retained damage observer's seven native identities/three registrations.
This is the previously owner-identified Plutonium r5354 build, not proof of
unmodified Steam behavior. No live map, stance or active player was observed.
The session owner's paused-state report does not establish a collision record.

External task0037 retains the task-specific driver, byte windows, decoder output,
synthetic checker and receipts. It reuses task0035's existing observer/profile and
Capstone dependency without modifying them. Only query/read process rights0x1010
are opened; each invocation qualifies before and after and closes in `finally`.
There are no inputs, writes, hooks, injection, suspension or process control.

Discovery begins with raw relative-call references to the already qualified
initialization/world wrappers, not guessed player offsets. Three executable scans
each cover RVA0x1000 through0x74f980: 7,661,952 bytes per scan. A fourth scan of
that same range accompanies three exact name leads, `player_standHeight`,
`player_crouchHeight`, `player_proneHeight`. All four code hashes agree:
`e58960235375ad90c26935e85c27f151c6a0f35374ece8070775f4e63d222234`.
The name search examines the prior qualified source spans starting at RVA0x76a000
(1,886,735 bytes) and0x937000 (4,194,304 bytes); none of the three exact names is
present there. This bounded absence is not absence from every module/build.
Search buffers are discarded; raw whole-image scans are not retained.

Total discovery scan volume is30,647,808 executable bytes plus6,081,039 data
bytes, including repeated scans. Eight successful process invocations read
36,832,978 bytes including qualification/repeats. The largest invocation reads
13,752,247 bytes; the per-invocation limit is24MiB, individual reads at most4KiB,
with readable-region admission. Sixteen initial caller slices, three selected
windows and five further caller slices total6,832 interpreted window bytes,
including overlap/padding. At most20 distinct candidate function contexts plus
three retained chain functions are considered, below24/1MiB. Partial slices do
not establish complete functions or authentic instruction boundaries by themselves.

## Direct observations

The initial scan finds16 raw E8 candidates for initialization wrapper0x1a9780 and
two for point-world wrapper0x2db280. Selected complete function decoding admits:

| RVA | Complete bytes | SHA-256 | Observed request role |
| --- | ---: | --- | --- |
| 0x260810 | 613 | `96f92624737f41756d081cd6589cf0dadc2949a9f0e29053318138a8fa7176ae` | Explicit mask/bounds forwarding wrapper |
| 0x0e7cc0 | 511 | `6fb1a4406577ca76a0eb2f9feb51fd3a11b57437538400daa5db23719eab4c9e` | Bounds forwarding with mask from filter record |
| 0x2fa5e0 | 125 | `1b98e4e827e89ad3ed69a7e07b46961037c145e0a710b357d3a1b38586d068f9` | Producer of the filter record supplied to0x0e7cc0 |

Argument positions below describe the observed x86 stack forwarding, not a
proposed Rust ABI or imported symbol names:

- 0x260810 receives `(result, start, lo, hi, end, excluded_index, mask, filter)`.
  Its call at0x26083b supplies `(result, start, end, lo, hi, mask, filter)` to
  0x1a9780. It does not resolve stance or replace the supplied bounds/mask before
  that call. Later entity-family work is not interpreted as player behavior.
- 0x0e7cc0 receives `(result, start, lo, hi, end, filter)`. Its call at0x0e7ce7
  supplies `(result, start, end, lo, hi, filter.word0, filter)` to0x1a9780.
  The bounds are forwarded unchanged; mask is the supplied record's first u32.
- 0x2fa5e0 receives `(result, start, lo, hi, end, excluded_index, mask, filter)`.
  It stores the supplied mask in `filter.word0`, associates a temporary exclusion
  record through `filter+0x0c`, then calls0x0e7cc0 at0x2fa654 with the five
  result/vector arguments and that filter. The exclusion record carries the
  supplied index and a derived model discriminator: index0x3ff or model word0
  gives-1; otherwise the model word gives `model-1`. Its additional word is
  0x01010001. These are filtering observations, not standing-state predicates.

The native chain for these selected calls is therefore bounds wrapper ->
0x1a9780 -> builder0x2d8770, the same builder qualified in
[findings0024](0024-original-brush-trace-consumer.md). The initialization wrapper
does not substitute a player shape. For moving centered endpoints, the existing
builder's scalar-f32 centering/radius/vertical-support construction applies to
the explicitly supplied bounds. This proves a shared request construction route;
it does not prove that an ordinary player uses either selected caller.

The later candidate checks show zero-extent point requests, fixed auxiliary
extent pointers and caller-owned bounds. Other slices show vector addresses
formed from an object at+0xc8/+0xd4. No inspected control flow connects that
object to the qualified local player or selects its ordinary standing case.
Those offsets, auxiliary pointers and encountered masks are excluded from any
player contract. The known local-player identity route does not supply collision
field provenance. No mutable player fields were sampled.

## Interpretation and unresolved contract

The observations support keeping extent endpoints and mask caller-owned, using
the existing moving-brush request abstraction. They do not support a new shape,
player component schema or stance API. Function proximity, an entity index,
matching familiar engine constants and height-related names would not qualify
standing movement; none is used to fill the gap.

The standing contract remains unresolved in all of these fields: lower/upper XYZ
values, their source record/constants, ordinary/alive/standing eligibility,
selected contents mask and its modifiers, movement caller/callback association,
and coordinate/origin convention. Capsule-like construction is established for
the selected bounds routes, while its standing-player use remains unproved.
Crouch/prone/dead/vehicle/actor distinctions are therefore also not established.

The discriminating next investigation should start from a qualified **player
movement update owner/callback**, with a source-backed association to the known
player identity, then follow its selected trace call and bound writes. Retain
these wrappers as downstream anchors. Merely reading model/auxiliary vectors at
the observed callers, scanning player floats or importing familiar hull sizes
would not resolve provenance. A pinned T6 movement declaration/symbol source or
qualified original update chain could supply that missing anchor. If obtaining
it needs session actions or a new native execution facility, that is a separately
authorized task. Until then the precise next Rust scope remains explicit-input
collision work; this task authorizes no standing-player defaults or integration.

## Validation and closure

All24 new windows re-read identically twice at terminal qualification. Their
covered pages are committed image pages, allocationRVA0, protection0x20.
Three complete functions decode to their final returns; direct jumps land on
in-function instruction starts. Terminal reads also match the retained complete
initialization wrapper, builder and moving narrow phase identities.

External `request_checks.py` interprets only selected forwarding instructions
against synthetic stack/record values and compares them to independently stated
argument contracts. Nine checks pass: explicit/record masks0,0x80000001 and
0xffffffff; excluded-index0x3ff, zero model and positive model branches.
This checks argument/mask/filter transcription, not original execution, standing
parameters or native trace outputs. An initial helper filename shadowed Python's
standard `inspect` module; import failed before opening an original process and
was corrected by renaming the task helper. No admission checks were relaxed.

Unchanged Cargo gates, gameplay/native output comparison, mutable sampling and
performance checks were not run for this documentation/external-only task.
All owned helper processes completed; all opened original handles closed. Prior
artifacts and pending task0026 files are preserved. No production code change,
commit, push or original operation remains running.
