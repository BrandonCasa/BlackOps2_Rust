TASK:
Complete all 1,219 todo.md tasks according to the accepted complete solo Origins plan.

STATUS:
PARTIAL - 205 completed; 1,014 remain. Full goal active.

FINDINGS:
Native entities and retained scripts/table identify primary wall buys. Origins changes MP40 pricing after the common table: 1,300 purchase and 650 ammo. Ordinary gameplay now supports interaction-driven primary purchases/refills and selected-slot replacement. Static skinned wall models exposed the renderer's blanket skin invalidation; actual mesh/joint-pose comparisons preserve reuse while changed poses invalidate nearby shadows. Capsule enemies remain temporary.

EVIDENCE:
External inspection/solo-content/window-wall-weapons-pose-cache/wall-weapons.json and verification.json passed. Normal Round 1 movement/combat reached Round 2; subsequent enemy-free weapon handling used controlled points at original map placements. M14 held fire consumed one round, MP40 consumed six, Remington consumed one shell with four generated pellets, B23R consumed three rounds. Four purchases, reloads/refills, direct/cyclic switches and magazine-reload switch rejection passed. Insufficient funds were rejected; each held refill charged once (250/650/450/500). Source viewmodel/wall frames were inspected across M14/MP40/Remington/B23R. Final room 51.11 FPS and combat 33.97 FPS at 1280x720; all 331 shadowed lanterns, 4,945,871 reused cube faces, 8,900 reused directional cascades, two allocation changes. Restart returned to Round 1.

CHANGES:
Reusable interaction/economy modules, typed purchase commands/rejections, accepted-item charging, duplicate-use latch, selected-slot weapon receipt and shared-pool refill API. F held-use input; fixed-step interaction ordering follows movement. Candidate reach/direction/sight queries, prompt/price/action feedback and hold/cancellation scaffolding. Preparation emits 18 primary wall placements/actions, original world-model references and native price precedence, preserving unrelated catalogs. Wall models reconstruct missing native probe fill with exposure-weighted textured radiance; shared viewmodel materials remain independent. Actual-window coverage extends the existing executable through native wall purchases and input-driven handling. Static caster bounds/transforms and skin joint matrices compare actual values; no blanket skin epoch invalidation. Performance receipt waits until the final survey/restart stage.

VALIDATION:
Full runtime-wall-weapons.json source/destination closure passed: 1,628 files, 1,409 PNGs, 1,037,986,224 bytes. Locked metadata/release-build/format gates passed; affected compiler/format gates repeated after actual fixes. Initial Debug/borrow compiler failures were corrected. First wall run failed because Round 2 enemies killed the player during reload; later weapon segment explicitly isolates enemies after normal combat. Next run failed targeting MP40 after floor settling; driver reaims and chooses viable approaches. Early price run loaded the pre-override pack during import; final runs awaited corrected pack and proved 650 refill pricing. Initial wall models were dark; exposure-weighted fill corrected black/washed-out presentations. New skins initially disabled all cache reuse (about 2 FPS), then false pose-change invalidations limited room performance to 22 FPS; actual-value comparisons restored final results above. No tests, fixtures, Clippy, replacement toolchains, delegation or push. Diffs/candidates reviewed before integration.

UNRESOLVED:
Offhand wall equipment and upgraded/shared-variant refill matching are pending; full wall placement/display/ammo checklist acceptance stays unchecked until those paths exist. Current wall placements use the existing start-zone placeholder; proper zones/doors/power requirements remain pending. Non-weapon interaction handlers, nonzero hold-action live coverage, remapping, downed interaction coverage and refunds need their associated systems. Segmented reload start/end/interruption, reload-completion transfer, burst interruptions, melee windup, penetration/projectiles, effects/audio and remaining arsenal gameplay are unfinished. Reconstructed wall fill is 90 exposure-weighted textured nits, not recovered native probes. Viewmodel FOV/fill, endpoint ADS interpolation, 4:3 FOV conversion, 0.01 recoil scale, movement spread and capsule hit regions remain practical approximations. Animated enemy shadow bounds/behavior still need actual enemy integration. Full quest/survival/8,000 Hz hardware/final playthrough gates remain pending.

RECOMMENDED NEXT WORK:
Complete reload/melee/burst action semantics and original enemy rigs; extend natural acquisition through Mystery Box and Pack-a-Punch with upgraded/shared wall refills. Build the source-defined zones/doors and general interaction handlers, then powered systems and equipment. Continue the complete checklist sequentially, retaining actual-pose shadow invalidation.

SCOPE NOT IMPLEMENTED:
The 1,014 unchecked tasks and final complete solo quest/survival acceptance. Current acceptance is normal Mauser combat plus bounded primary-wall arsenal handling, not complete Origins.
