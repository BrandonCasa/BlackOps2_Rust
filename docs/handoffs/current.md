TASK:
Merge audio source/attenuation/purchase fixes into main while preserving every main commit since the worktree base.

STATUS:
COMPLETE: reviewed every main commit since base, preserved its behavior and validated the combined merge for local main integration. No push.

FINDINGS:
Base is 8e585b9. Main added exactly cee19b3: grenade release delays, held prime poses, shared viewmodel clocks and native shot tails. Its complete twelve-file change was reviewed, including clean merges. Audio action notes must retain those clocks while using explicit owner attachments. Assets must retain release_delay and camera-origin equipment offsets alongside native distance_min.

EVIDENCE:
Native audio reference and the original six-window receipts remain in external audio-recovery. Integration receipts use merge-main-* names; merge-main-summary.json retains the reviewed commits and combined results. A semantic comparison confirmed combined game.json equals reviewed main after removing the 2,510 new minimum-distance fields; all five main-only source/tool files remain byte-equivalent. Shared audio, equipment, contract and verification diffs were reviewed independently of conflict detection.

CHANGES:
Preserve all cee19b3 presentation, weapon, equipment, offline preparation and verification behavior. Add typed World/Entity audio sources, current transform/offset tracking, native minimum dry distances, one attenuation curve, normalized spatial coordinates, persistent out-of-range loops, owner retirement and restart cleanup. Restore one door/debris purchase cue and board-attached repairs with capped player reward cues. The handoff was the only conflict; this current integration record replaces both retired records. Main content and source interactions were reviewed even where Git merged cleanly.

VALIDATION:
Before integration, six audio windows passed 106,901 source frames with zero source/gain/backend-scale errors, including all eleven doors, eighteen repairs, native generators, Monkey/Semtex, pause/settings and restart. Combined locked metadata/release/fmt and diff checks passed. The combined 2,597-file closure passed at 1,140,678,549 bytes with 1,461 PNGs and all 2,596 staged LFS pointers/objects. Combined source/Semtex/wall-weapon windows exited zero: 1,730/13,359/21,009 checked source frames, zero source/gain/backend-scale errors, owner retirement, pause/settings and restart. Native 130 ms Semtex release delay, held 866.667 ms prime endpoint and 316.667 ms Mauser shot tail were retained. M14/MP40/Remington/B23R firing, magazine/shell reloads, interruption, switching and burst cancellation passed; all 46 measured weapon clocks matched requested times. Main remained clean at cee19b3 before integration. No compiler/runtime failure occurred in these combined runs. Shared validation-slot contention delayed equipment checks until other processes exited. No unit tests, suites, fixtures, Clippy, computer use or concurrent validation windows; originals remain untouched.

UNRESOLVED:
Subjective repair/original-game listening, exact native acoustic/blending parity and sustained manual play remain unqualified. The earlier pre-merge source window logged secondary UV renderer warnings; this merge does not repair them. Combined checks exercise controlled paths and do not establish full original equivalence.

RECOMMENDED NEXT WORK:
Continue absent gameplay through the shared application; obtain ordinary repair listening feedback. Preserve the native action clocks when adding future audio timelines.

SCOPE NOT IMPLEMENTED:
Absent gameplay audio, native reverb/ducking, exact original rendering/acoustic equivalence, unrelated rendering fixes or full-map sustained survival.
