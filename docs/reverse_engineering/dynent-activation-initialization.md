# Dynent count capacity and initial server activation

Status: **INVESTIGATING**. The 19-versus-275 count distinction is source-qualified.
This provides a conditional initialization recipe, not an observed current scene
or proof of the player's collision receiver binding. Original execution, process
reads and emulation were not used.

## Evidence and reproduction

Inspect the configured native disk image with SHA-256
`2aff340f22f0c68bf46511c7bbc6167e4bb84524817f54cae08e8f60e6f9afc3`
and PE image base `0x400000`. External `movement-dynent-02/refs.py` searches only
the specified count/array/bank address literals for bounded candidate references;
such decoded windows are leads, not admitted function entries. Then
`native_qualify.py` follows reachable bytes from complete entry boundaries and
verifies every byte against that image. Aggregate receipt SHA-256:
`d228b091d76b7be1a1047651859e4542edf0599a5b036c663ac6fd4b541d47df`.

| Entry RVA | Role | Instructions / bytes |
| --- | --- | ---: |
| `155140` | Definition construction, class counts and allocations | 768 / 2711 |
| `4055f0` | Definition sort comparator | 48 / 109 |
| `4058c0` | Parsed definition/placement constructor | 289 / 933 |
| `0a99e0`, `1f39b0` | Aligned allocation and requested zero initialization | 86 / 252 |
| `0331f0`, `152180` | Server initialization and u16 count getter | 100 / 298 |
| `1fc400` | Fixed sector-bank initialization | 45 / 157 |
| `1a9220`, `10acc0`, `2eed80` | Model bounds/radius access boundaries | 38 / 131 |
| `405dd0`, `406000` | Model/brush definition bounds and server linking | 255 / 1075 |
| `2227a0`, `404430`, `404380`, `404490` | Link, ordered insertion and sector allocation | 362 / 1164 |
| `2e39b0` | Unlink, contents repair, free-list and active flag repair | 140 / 428 |
| `296930` | Conditional state-transfer count mutation | 252 / 826 |

These 19 complete bodies total 2,383 instructions/8,084 bytes. Decoding has no
unresolved instruction; the allocation body's indirect call at `1f3a6e` through
VA `b6a228` remains an unqualified memory-commit dependency. It is not invoked.
The routine nevertheless exposes its subsequent explicit zero-initialization
request; this is not a claim of successful runtime allocation.

Rejected leads: `0331e0` overlaps preceding instructions; `296940` and `27eeb0`
are interior entries rather than function starts. Only corrected `0331f0` and
`296930` contribute to the receipt. A direct call instruction at `27eedb` targets
`0331f0`, but its enclosing startup owner was not qualified. No direct relative
call to `155140` was found in the scoped `.text` scan; this does not exclude an
indirect/encoded caller or establish that the builder runs for this session.

Retained observations reuse the pointer-free dynent01 metadata, 12,515 bytes,
SHA-256 `6c6ab9bc7610e1afe6a656e335981a35f474304c0f085c0f1e9f2bc3ecba4266`,
from the licensed 78,753,344-byte Origins fastfile with SHA-256
`663c2f6e275fb6a8a2844bcc0e31808abe681ce7c2a2ddc6f616ef98fc7cd0b5`.
No second asset extraction or original mutation occurred in this task.

## Count, class and capacity ownership

The native global clipmap base is VA `2503880`. These fields are distinct:

| Field | Native address / width | Meaning established here |
| --- | --- | --- |
| Original class-zero count | `250397c`, u16 (`+fc`) | Parsed class-zero count before reservation |
| Count slots 0/1 | `250397e + 2*c`, u16 (`+fe`) | Client-side count/capacity initializer |
| Count slots 2/3 | `2503982 + 2*c`, u16 (`+102`) | Server-side initial definition population |
| Definition pointers 0/1 | `2503988 + 4*c` | 84-byte records shared by both sides |
| Pose/client pointers 0/1 | `2503990` / `2503998` | Client-side arrays sized by count slots 0/1 |
| Server pointers 0/1 | `25039a0 + 4*c` | Eight-byte records sized by slots 2/3 |
| Collision pointers 0–3 | `25039a8 + 4*bank` | 32-byte records sized by the corresponding slot |

`155140` clears the counts/pointers, obtains the parsed definition count, and
requests `84*(parsed_count+256)` bytes for definitions. Its input threshold emits
an error request when parsed count is at least 4096; the external error owner's
termination policy is not admitted, so safe admission must enforce its own cap.
The builder invokes a sorting routine with `4055f0` before publishing class bases.
The comparator orders model presence, then a boolean from the definition-type
table, then model pointer ordering or brush selector difference. This is **not**
a portable stable sort identity: the model comparison even returns positive for
equal pointers. Preserve retained ordinals/associations; do not synthesize them
by sorting names or Rust asset IDs.

For each parsed definition, nonnull XModel `+20` chooses class zero and sets flag
`0x200`; otherwise class one sets `0x400`. First encountered class records publish
their pointer into the corresponding definition base. Each record increments
both the u16 client count `c` and server count `c+2`. If class zero is empty its
base points to the end of parsed storage, reserving the append region there.

At `15588c–1558a7`, the builder copies class-zero count to original-count `+fc`,
then explicitly adds **256 only to slot zero**. The appended 256 records receive
four constraint words `0x1ff` at `+3c/+3e/+40/+42`; this is reserved storage, not a
bit-packed count interpretation. Allocation requests then use:

```text
client pose[c]: 32 * count[c]
client state[c]: 20 * count[c]
client collision[c]: 32 * count[c]
server state[c]: 8 * count[c+2]
server collision[c]: 32 * count[c+2]
```

For the retained map, original count 19 and slots `[275,0,19,0]` therefore match
the native recipe. All 275 decoded class-zero definitions are storage records;
the **initial server** population is ordinals 0–18. Slot two is server count for
class zero, not a third independently selected definition class. No 255 mask or
encoded-count rule is needed.

Counts are mutable runtime fields. In the zero-branch state-transfer routine
`296930`, `296a01` overwrites client count slot zero with original-count `+fc`.
That proves the initialized capacity/count value cannot be treated as an
immutable current population. The rest of its stream/activation dependencies is
outside this task; it does not establish a current 19-record client scene.

## Server initialization, pose input and valid initial ordinals

`0331f0` loops classes zero and one, initializes sector bank `c+2`, reads count
`c+2` through `152180`, and loops unsigned ordinals strictly below that count.
For each ordinal it uses definition `base[c]+84*i`, server `base[c]+8*i` and
server collision `base[c+2]+32*i`. It writes server flags `u16=1`, health from
definition `+44`, and clears collision sector/next together. Consequently this
producer does not index the 256 client-only reservation through the smaller
server allocation. Later generic link/query routines have no independent range
guard; an adapter must carry the producer's side/count binding rather than index
all definitions through every array.

Class zero ordinarily calls `405dd0`; class one calls `406000`. A class-zero
record may instead clear server flags and unlink when two supplied predicates
(`2f1650([VA2114604])`, `297610()`) are true, its ordinal is even, all four
constraint IDs are `0x1ff`, and definition flag mask `0x1` is clear. Predicate
values/owners are not qualified. Retained first 19 definitions all have flags
513 (`0x201`), so their force-link mask `0x1` blocks that skip branch regardless
of those two values. **If this initializer runs with valid map/provider bounds**,
each of those 19 takes the model-link path. This is a conditional native recipe,
not an observation that this session has those linked sectors.

Model linker `405dd0` obtains model bounds, builds quaternion rows from definition
placement `+4`, transforms bounds with SIMD sign selection and adds definition
origin `+14`. Brush linker `406000` obtains bounds from definition selector `+28`
and performs the corresponding supplied-placement construction. Both call
`2227a0` with bank 2 or 3 and the same definition ordinal. They do not read
client runtime pose arrays or rewrite the definition placement. The parsed
constructor `4058c0` writes definition origin from its parsed input and supplies
parsed angles to `265050` to write its quaternion; angle conversion itself is
not newly qualified here. Retained serialized placement is available, but later
pose/definition writers and activation events must still be separately bound.

## Sector links and activation state

The sector storage is **inline**, not a pointer read: bank header
`VA17ff790 + bank*0x501c`, followed by node pool at header `+1c`. This corrects
the earlier dynent01 formula to
`VA17ff7ac + (class+2)*0x501c + node*20`.

`1fc400` runs only if clipmap field `+4` is nonzero. It requests clearing the
entire bank, obtains selector-zero model bounds through `1a9220`, initializes
root node one, and free list 2 through 1023. There are 1,024 node slots total;
zero is the terminator. Root split chooses the greater X/Y extent (Y on ties),
then the midpoint `(min+max)*0.5`; no Z split is selected here. `404380` allocates
a child only when the chosen X/Y extent is strictly greater than disk constant
512 and the free list is nonempty. It updates the free-list head and node axis/
midpoint; missing capacity leaves the candidate at an existing ancestor.

`2227a0` maps bank to definition class with `bank & 1`, stores definition contents
in the selected collision record and ORs masks down the traversed node path. It
chooses the containing sector from supplied bounds, unlinks when ownership changes,
stores all six bound words and calls `404490` to redistribute wholly sided links.
`404430`/`404490` insert **one-based ordinal links in ascending ordinal order**;
movement query ties therefore consume source ordinal order rather than insertion
or Bevy entity order. Bounds crossing/touching a split stay at the parent; strict
separation can descend. Server banks set flag mask `0x2`, client banks set `0x4`.
Queries use actual links and masks, not merely those flags.

`2e39b0` removes the matching link, sets its sector to zero, repairs parent/child
and free-list references, recomputes ancestor contents from child masks and
remaining record contents, then clears server flag mask `0x2` or client `0x4`.
These producers provide the activation mutation boundary, but their callers and
current inputs are not exhaustive active-scene evidence.

## Validation and remaining binding

External `source_checks.py` passed 50 synthetic reconstruction checks: client/
server count separation and storage sizes, class/bank mapping, initial 19-record
server bounds, all retained force-link gates, sorted one-based insertion under
reversed order, link/unlink flag masks, node capacity, 512 boundary and midpoint.
No native code was invoked; these do not establish `MATCHED` or `VERIFIED`.

Still required: qualified world startup owner of `0331f0`/serialized global
binding; current bank/map bounds and current links; all later definition placement
or contents mutation, reserved client record activation and side-specific count
updates; successful provider/bounds admission including collmap; and player
receiver/hull query ownership. A reproducible initialization profile can now bind
server class-zero range 0–18 **conditionally** using admitted definition flags and
source counts. Missing current activation/pose inputs must remain explicit errors.
No zero suffix or empty decoded runtime array is a full-map exclusion.

See [dynent consumer](dynent-collision-contract.md),
[global ownership](collision-global-ownership.md),
[architecture](../architecture/README.md) and
[handoff](../handoffs/movement-dynent-02.md).
