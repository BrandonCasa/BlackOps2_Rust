# Current handoff

TASK: Fix all fifty findings in docs/static-review.txt with one local commit per fix, then resume the TODO list. Locomotion TODO completed and committed at 23d9214d4.

STATUS: PARTIAL

FINDINGS: Review finding04 complete; 49 findings remain. Lighting migration discarded brush_entity while rebuilding node extras from only static_instance. The actual canonical map loses all 67 brush roots under the previous operation and fails its scene contract. Corrected stripping retains every nonlighting node member, removes only lighting, and preserves legacy static-instance conversion. TODO counts remain 479 complete/734 open of 1,213.

EVIDENCE: E:/Coding/Captures/static-review-04-20261008/asset-review.json records original map SHA256 bc702dee636a539d48b6af4f6f0a297aefa9008b84616c64da49d5f5ff7d4709, reproduced old brush scene root count mismatch, corrected 67-root acceptance, unchanged realtime document and idempotence. unchanged-closure.json confirms all 4,137 runtime hashes still match the fully qualified locomotion import.

CHANGES: tools/realtime_assets.py preserves nonlighting node extras instead of replacing them wholesale. Updated the report's fix status, current project state and this single handoff. Concurrent actor-rendering/shadow edits in Rust and their game run are unrelated and left untouched; none are included in this fix.

VALIDATION: Changed Python source parsed. Recovered actual map/content supplied all brush data; no synthetic inputs or fixtures. Previous strip reproduces failure; corrected strip passes existing native brush scene contract and is identical to the already-realtime source document on repeat. Initial diagnostic chose the wrong brush-catalog key; corrected to environment.brush_entities. Complete runtime hashes are unchanged, preserving the previous full semantic closure/LFS validation. No runtime assets/Rust/dependencies changed in this commit; compiler/format/game-window gates were not rerun. Locomotion parent passed locked metadata/release/format, 210 focused clip/variant cases, ordinary six kills/Round2/seventeen views/restart, audio and zero streaming errors. No tests, Clippy or push.

UNRESOLVED: Review findings01-03 and05-50 remain. Full migrate still has reported streaming-chain pruning and hardlink write issues (05/06), so this correction qualifies the actual metadata-strip/brush-contract path without running that unsafe complete migration. Concurrent actor-texture/shadow work remains dirty outside this scope. 734 gameplay TODOs remain; native RNG/per-clip speed fitting, crawler/stumpy/super-sprint consumers, performance and sustained manual survival remain unqualified.

RECOMMENDED NEXT WORK: Fix independent preparation finding05 (retain texture-streaming.json full-chain dependencies), then06 (atomic replacement prevents writing through source hardlinks). Return to renderer findings01-03 after concurrent renderer work commits. Finish all fifty review findings before resuming Generate missing combat clips for recovered rigs.

SCOPE NOT IMPLEMENTED: No streaming-pruning/hardlink correction, other review findings, actor rendering changes, runtime-pack migration/import, new gameplay TODO, performance benchmark or release-package refresh.
