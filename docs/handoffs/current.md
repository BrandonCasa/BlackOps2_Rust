TASK:
Fix native ADS positioning from local main in codex/fix-ads-positioning. Final user instruction: stop validation and commit this worktree.

STATUS:
PARTIAL: source-backed positioning implemented and committed locally; rendered acceptance stopped at the user's request. No push or main merge in this final step.

FINDINGS:
Recovered engine functions remove FOV positioning bias at full ADS, step a clamped ADS fraction using reciprocal native transition times, blend native horizontal 4:3 FOV using endpoint-dependent zoom fractions, and compose an animated camera tag separately from the base weapon frame. Current five ADS-up clips animate tag_torso. The former PostUpdate projection change could run after Bevy's camera update: identical camera-space points then returned different projected pixels. Explicit ordering fixes this. Sight sockets are diagnostics; optical points follow the actual rigid skin joint and inverse bind.

EVIDENCE:
E:/Coding/BlackOps2Research/ads-native retains Ghidra's NativeFocused project, exported pseudocode/callers, hashes, coordinate conventions and preparation measurements. Original executable SHA256 f6f7104af2bd0c2b931ea1e739969e85845e499affbf686144c6b490b344ab1e; offline unpacked research input SHA256 ebf8b363455c22ed25a323a3ba0a45318cc0dd09769d4ec0ba3fbbac7d01251c. Engine addresses: 00533090 positioning bias; 005332e0 transform assembly; 00539990 viewweapon update; 008e1500/008e7b00 ADS stepping; 004a0890 ADS FOV; 005303e0 endpoint state; 008e9470/008e94e0 zoom fractions; 004a10a0 projection; 004a0730 camera-tag composition; 008ed6c0/008ed300 look-lag suppression. Originals remain untouched and the research executable was never run. Rendered evidence is external in ads-window-01, ads-window-02 and ads-window-03.

CHANGES:
Native ADS curves and zoom fractions in offline preparation/runtime contracts. Shared world/viewmodel full-ADS lens, fading hip calibration, animated camera eye with inverse model-frame composition, bounded geometry-derived optical calibration and native look-lag suppression. Projection changes precede camera updates; audio maintenance follows animated camera positioning. The window driver covers five weapons, both aspect ratios and transition/fire/reload/switch/pause returns. Reviewed every main change since base cee19b3, including clean merges: a489cea, 08ef6f1, 5e7c6af and c6d1363. Incorporated audio ownership/attenuation/purchase cues and GPU preprocessing without replacing their behavior; preserved native grenade and shot clocks. Continuous validation updates use main's shared policy.

VALIDATION:
Locked metadata, combined release build and formatting checks passed. Final source/runtime/staged-LFS integrity passed: 2,597 files, 1,461 PNGs, 1,141,169,517 bytes. Initial encrypted analysis yielded stub-only code; ordinary/8 GB Ghidra analysis exhausted heap; focused analysis recovered 23,590 functions. Initial Rust decimal-literal compile failure and research script startup/locking/address parsing failures were corrected. Window 01 exposed uncalibrated optical offsets; window 02 completed 120 captures with optical alignment passing but scenario checks failing on entry capture and stale projection returns. Window 03 used corrected skin-bound measurement, ramp capture and projection ordering on combined GPU/audio main: 72 of 120 cases passed, including all five weapons at 16:9 and Mauser at 4:3. Thirty settled samples had maximum optical error 0.015808 pixels and zero measured return error. User stopped this owned window and its waiting helper before equipment regressions began. Every launch checked the shared slot and waited for its owner; other task instances were never terminated. No unit tests, suites, fixtures, Clippy or computer automation.

UNRESOLVED:
The remaining 48 corrected 4:3 cases and grenade/Semtex/Monkey/Claymore regressions were not run to completion, per the user's stop instruction. Remington's visible obstruction is excluded from this positioning task at the user's direction; another task owns weapon assembly. Its projected reference passes but does not establish unobstructed visible iron alignment. Hip lens/framing, optical correction and procedural motion retain tuned reconstruction. Reverse ADS-up sampling on exit, complete native blend/tick arithmetic and variable-zoom/attachment branches are not established. Full rendered acceptance and exact native equivalence are not claimed.

RECOMMENDED NEXT WORK:
When requested, finish the remaining rendered/equipment checks, incorporate the separate assembly fix and review any newer main changes before local integration.

SCOPE NOT IMPLEMENTED:
Separate weapon assembly fixes, absent weapon/movement variants, full native blend arithmetic, original pixel equivalence, unrelated gameplay, sustained full-map survival or main integration after the final stop-and-commit instruction.
