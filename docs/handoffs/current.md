TASK:
Fix grenade throwing and the shared viewmodel animation path using the installed original reference, commit this worktree and merge it into main.

STATUS:
COMPLETE: scoped presentation fix; compiler/asset gates and all five focused actual-window acceptance checks passed. Integration branch: codex/fix-grenade-throw; local main target, no push.

FINDINGS:
The reconstructed equipment offset moved authored shoulders in front of the eye. Cooking replayed the unprimed idle instead of holding the pull-pin endpoint. Bevy 0.19.1 wraps an exact end-time seek even at zero speed; paused active animations still evaluate curves. Primary fire presentation ended at cooldown: Mauser fireTime is 96 ms, while its source clip lasts 333 ms. Raise/lower and melee lacked a shared gameplay-owned clock. Native throwable fireDelay was omitted.

EVIDENCE:
External viewmodel-native-audit records executable/source hashes, native weapon fields, retained OAT frame/channel contracts and the Bevy evaluator finding. Installed references contain compiled code/assets; the native DObj/XAnim evaluator was not recovered. Window-grenade-before shows the exposed sleeve; final pin-pull, throw, Monkey winding and Claymore placement captures were reviewed. Final requested/actual clock maxima match for Frag/Semtex, Monkey, Claymore and primary weapon actions, including both 866.667 ms held prime endpoints and the 316.667 ms Mauser fire tail.

CHANGES:
All four equipment rigs use the shared camera origin. Cooking holds the prime endpoint. Frag/Semtex/Monkey release delays are 120/130/150 ms, with continuous throw playback and cooked fuse progression. Timer-driven players stay paused while sampled. Primary raise/lower/reload/melee use shared clocks; fixed-step shot age retains native clip tails until interrupted. Audio uses those primary clocks. Offline preparation, content validation and game.json agree. Existing window drivers record actual/requested clocks and bounded phase captures. Originals remain untouched.

VALIDATION:
Locked metadata, release workspace build, formatting and diff checks passed. Offline import validated 2,597 runtime files; staged inspection verified all files and 2,596 LFS pointers/objects. Final Frag, Semtex, wall-weapon, Monkey and Claymore windows exited zero with passed receipts and restart checks. M14/MP40/Remington/B23R firing, magazine/shell reloads, interruption, switching and burst cancellation passed. Equipment windows covered cooked damage, bounces, sticky surface/moving-actor attachment, lure/detonation, mine limits and triggering. Controlled inventory/positions were used where stated in receipts. Twenty tracked Markdown files; no unit tests, suites, fixtures or Clippy. Initial Python version and tuple-reporting mistakes were corrected. The first Semtex attempt ended without a completion receipt/diagnostic; the logged retry passed.

UNRESOLVED:
Exact native blending, action-rate arithmetic and camera-lens parity remain unqualified. Authored held/late poses can leave the view. Full-map manual/sustained play, every weapon variant and bottle/movement presentation were not qualified by these focused windows.

RECOMMENDED NEXT WORK:
Continue absent weapon/movement presentation through Bevy using native source tracks, with focused real-window comparisons.

SCOPE NOT IMPLEMENTED:
Replacement animation infrastructure, exact original pixel/arithmetic equivalence, complete absent arsenal/movement/ADS variant playback, held-death equipment drops or unrelated gameplay/rendering features.
