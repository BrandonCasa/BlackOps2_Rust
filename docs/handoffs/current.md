TASK:
Complete all 1,219 todo.md tasks according to the accepted complete solo Origins plan.

STATUS:
PARTIAL - 154 completed; 1,065 remain. Full goal active.

FINDINGS:
Normal gameplay now uses an owned Mauser, native ammunition/damage values and the original animated Mauser/Dempsey rigs. Weapon ownership/actions are separate from round state. Original socket attachment needs compensation for OAT's root basis. First-person meshes need a separate projection, conservative visibility and isolated lighting. The existing shadow cache's single-camera binding assumption disabled reuse with an overlay camera; selecting the world-layer binding restored reuse. Capsule hit regions approximate anatomical targeting until animated hit volumes exist.

EVIDENCE:
External inspection/solo-content/window-weapon-cache/verification.json passed movement, combat, Round 2, one room view and restart. round-two.png visibly shows the textured original gun/arms and retained reserve ammunition; playing/round-two frames were inspected. performance.json: paused room 50.25 FPS, combat 36.02 FPS at 1280x720; 331 shadowed lanterns, 2,177,369 reused cube faces and two allocation changes during the bounded run. Earlier window-owned-weapons and socket/lit runs passed 15-view combat/restart surveys but exposed presentation defects. window-weapon-integration passed gameplay with a 2.3 FPS cache regression; final cache run corrected it. Existing runtime-arsenal-content.json hashes still match all 1,628 files exactly.

CHANGES:
Reusable owned weapon inventory, per-instance magazines, shared reserve pools, two-slot capacity, typed action/intent/emission interfaces, equip/switch/reload/normal-fire scaffolding and source-specific damage calculation. Player owns inventory; Session no longer owns pistol ammunition or silently replenishes reserves at round changes. Keyboard/wheel slot input, held fire and right-button ADS intent are collected. HUD reads selected weapon/ammo/reload state. Default Mauser combat uses native damage/head multipliers and current capsule hit regions. Bevy loads original rigs/clips, plays idle/fire/reload actions, attaches gun to tag_weapon, pauses playback and excludes first-person skins from world shadows. Bevy gltf_animation enabled; original meshes retain engine skinning. Separate 65-degree vertical viewmodel projection and fixed cool fill are reconstructions. Matching camera MSAA and explicit HUD target remove compositing ghosts. Verification aims at heads and records failure JSON for future failures.

VALIDATION:
Current asset closure matches the previously validated import, zero hash failures. Locked metadata/build/format gates passed; affected build/format checks repeated after fixes. Actual final game-window and performance outcomes above passed. Initial glTF-animation feature and system parameter-count compiler failures were corrected. Misplaced/unlit/clipped rigs, mismatched camera MSAA/HUD ghosts and the one-camera shadow cache regression were corrected and re-exercised. window-first-person-pass exited 1 without a receipt before failure JSON capture was added; its precise cause is unavailable. No tests, fixtures, Clippy, replacement toolchains, delegation or push. Integration candidates/diffs reviewed; current asset closure remains unchanged. Wheel events are consumed fully within each frame; affected compiler/format gates repeated after that correction.

UNRESOLVED:
Switch/equip, automatic/burst, shared-ammo and segmented-reload APIs exist but their checklist behavior tasks remain unchecked pending live coverage and remaining native timings (including burstFireDelay and segmented start/end). Magazines transfer at the current insertion point, so the explicit reload-completion transfer task remains unchecked. ADS intent has no aiming behavior yet. Melee windup, projectile delivery, spread/recoil, effects/audio and normal weapon acquisition are pending. Other characters/arsenal presentations remain unchecked. Temporary zombie capsules remain. Hit volumes, viewmodel FOV/fill and melee recovery are practical approximations, not native arithmetic/pixel equivalence. Full quest/survival/device/relocated-package gates remain pending.

RECOMMENDED NEXT WORK:
Complete and exercise switching/fire/reload/ADS semantics across weapon families, then bind original enemy rigs and action transitions. Use native burst/segmented timing fields and bounded real-window coverage. Continue remaining content/interactions/economy and full gameplay milestones sequentially. Preserve world shadow caching when adding actors/cameras.

SCOPE NOT IMPLEMENTED:
The 1,065 unchecked tasks and final playthrough gates. Current acceptance is bounded Mauser combat/render/restart, not a complete arsenal, quest or sustained session.
