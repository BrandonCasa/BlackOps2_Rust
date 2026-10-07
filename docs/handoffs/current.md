TASK:
Fix all currently implemented audio source ownership/offsets and duplicate distance attenuation; restore native door/debris purchase and capped barrier reward cues.

STATUS:
COMPLETE: implementation, compiler/asset gates and all six automated game-window workloads passed. Subjective listening remains unqualified.

FINDINGS:
Player calls supplied no owner; kill dialogue could originate at its victim. The importer discarded native DistMin, while linear and Rodio inverse-square falloff both reduced output. At three meters, board slam lost about 20 dB despite its native 5.715-meter full-gain radius. Native volume conversion was correct. Door transactions omitted the native purchase cue. Native scripts distinguish stationary door/debris purchase events from player-owned barrier rewards.

EVIDENCE:
Recovered installed-game _zm/_zm_blockers/_zm_utility scripts, native OAT alias CSVs and Rodio source establish source placement, native endpoints and duplicate attenuation. External audio-recovery/preparation.json, import-sources.json, index-sources.json and source-fix-summary.json retain preparation/hash/window evidence. Final window receipts are under window-sources-03 and window-{doors,barriers,monkeys,generators,semtex}-sources-01. Originals remain unchanged.

CHANGES:
Typed World/Entity sources preserve offsets and secondary ownership; current hierarchical transforms are resolved after animation and before Bevy propagation/audio playback. Persistent loops survive distance changes; owner removal stops loops and freezes one-shot tails. Prepared sound metadata imports native minimum dry distance with backward-compatible defaults. One distance curve and normalized Bevy spatial coordinates remove double attenuation, including pending sinks. All implemented callers are audited. Door/debris success emits one native cue; repaired boards use their animated anchor and awarded repairs emit player-owned cues. Existing real-window drivers now measure source poses, gains, native-radius return, owner retirement and purchase/reward counts.

VALIDATION:
Locked metadata, release workspace build and formatting passed. Full assembled closure passed: 2,597 runtime files, 1,461 PNGs, 1,140,678,459 bytes; staged closure and all 2,596 LFS objects passed. Native metadata-only comparison confirmed 2,510 minimum-distance additions and no sample changes. Source window passed movement, reload/dialogue/breathing and secondary tracking, gains of 1/0.5043/0/1 on the same loop, local offsets, pause/settings, owner retirement and restart. All eleven door groups passed exactly eleven buy events/sinks, rejection/held-use suppression, twenty mechanical events and restart. Eighteen barrier repairs passed board motion, reward caps, pause/settings and restart. Monkey lifecycle/combat and all 29 generator cases passed. Final Semtex/combat passed moving charge audio, player kill dialogue, stationary explosion layers, seven death-vocal sinks, Round 2 and restart. Source/door/barrier/Monkey/generator/Semtex receipts recorded 1,338/11,604/17,348/16,516/46,994/13,101 source frames with zero source/gain/backend-scale errors. These controlled drivers qualify exercised behavior, not full-map manual play. No unit tests, fixtures, suites or Clippy. Python selection, compiler branches/borrows and the initial metadata contract rejection were fixed. Two source-window runs exposed restart audio cleanup races; explicit restart ordering and idempotent loop/retirement cleanup fixed them, and the rerun passed. Copying the final executable while the owned Monkey window was running was refused by Windows; copied successfully after exit. Only one shared validation instance runs at a time; no other task's process is terminated and no computer use is performed.

UNRESOLVED:
Subjective repair loudness and original-game live listening comparison were not performed; actual native-radius gains and playback sinks are qualified. Source-window restart logged missing secondary UV material attributes; this renderer warning is outside the audio fix. Semtex capture logged one duplicate screenshot-target warning. Exact dry curves between native endpoints, panning, reverb/ducking and variant weights remain reconstructed or unrecovered.

RECOMMENDED NEXT WORK:
Listen to ordinary repairs and tune from user feedback; investigate renderer secondary UV attributes separately. Changes are local on codex/audio-source-fixes; no main merge or push was performed.

SCOPE NOT IMPLEMENTED:
Absent gameplay audio, native reverb/ducking reconstruction, exact original acoustic parity, full-map manual/sustained survival.
