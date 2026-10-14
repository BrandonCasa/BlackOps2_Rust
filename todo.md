# Origins first playable solo checklist

Deliver a real zm_tomb / Origins build that opens with a fresh runtime pack and supports solo survival in the original starting area. Initial acceptance covers rounds 1 through 5 with the source starting pistol, knife, grenades, ordinary zombies, barriers, points, health, solo revive/death, restart, essential HUD, and essential sound/animation.

This is an explicitly incomplete Origins build. Full-map survival and progression, the remaining arsenal, perks/powerups, generators and special enemies, digging/buildables, staffs, robots/tank, quests/finale, final rendering fidelity, cinematics, and release polish belong in [todo_final.md](todo_final.md). Do not claim those systems or later rounds are supported based on this milestone.

## Delivery boundaries

- Prioritize a usable scoped pack and startup, then the first working survival loop. Test completed paths as soon as their actual dependencies load; do not wait for every material family or final visual acceptance.
- Keep the real Origins world and authored starting-area geometry, placements, collision, spawn rules, mechanics, timing, and balance. Keep source-closed access closed; unsupported later content must be explicitly unavailable.
- Require the dependencies of the world, placed props, and enabled gameplay. Loading a shared Zombies file does not make every model, weapon, camo, attachment, image, or sound in it mandatory.
- Reuse the existing Rust builder, pack format, in-memory conversion/baking, loader, Bevy/Avian systems, and implemented gameplay. Do not remove working features solely because their remaining coverage is deferred.
- Existing documented PBR/bake approximations may remain for this milestone. Missing payloads, collision, gameplay data, visible enabled objects, or referenced dependencies remain blockers; generic material fallbacks and fake gameplay values are not authorized.
- Follow AGENTS.md for source locations, pack-only generated outputs, static linking, process/capture ownership, BRP scenarios, and scoped commits. This user-requested two-file split replaces the former single checklist. No subagents are authorized by this plan.
- The checklist split changes delivery priorities only. Implement the scoped builder/runtime requirements below before claiming that the existing validation gates accept this milestone.

## 1. Produce the initial Origins runtime pack

- [ ] Trace the dependencies of zm_tomb world geometry/collision, its placed static props, the source solo player, starting equipment, ordinary zombies, starting barriers/spawns, and essential HUD/audio.
- [ ] Restrict mandatory model/material/image/animation/sound selection to those dependencies through the existing builder; retain shared and patched definitions they actually reference.
- [ ] Resolve each retained dependency through the existing native zone/patch rules; prove non-use before removing a shared/test asset from the required set.
- [ ] Recover and pack the source-derived initial content and tuning needed by the actual runtime schema; do not borrow loose legacy/generated data.
- [ ] Complete conversion and pack bindings for every dependency of enabled initial content using the existing conversion paths.
- [ ] Explicitly exclude deferred content from the enabled game flow and remove only its now-unreferenced requirements from builder/runtime assembly. Keep one builder and runtime pack path.
- [ ] Separate explicit visual approximations from genuinely missing required data in the existing material/pack checks; retain precise errors for the latter.
- [ ] Ensure game.json, prepared/runtime.json, scenes, collision, model buffers, textures, animation clips, sounds, and UI entries needed by the initial game load and resolve inside origins.pack.
- [ ] Require FX/camo definitions only where enabled initial content actually consumes them; handle absent deferred systems intentionally rather than supplying fake required catalogs or placeholder payloads.
- [ ] Preserve the source-derived atmosphere already bound to the Origins world without making final graphics work a startup gate.
- [ ] Generate a fresh compatible origins.pack directly from the authorized Steam installation and pass build_assets check for the declared initial scope.

## 2. Player, equipment, and session behavior

The source-derived profile tasks below apply to the supported solo states and initial equipment.

- [ ] Convert the exact player spawn position and initial yaw.
- [ ] Convert source player movement, stance, mud, sprint, dive, and downed tuning required by runtime.
- [ ] Complete rounds.gait_speeds.
- [ ] Complete solo_revive.downed_height.
- [ ] Complete solo_revive.downed_speed.
- [ ] Preserve source solo-revive timing, limits, downed weapon selection, and recovery behavior.
- [ ] Bind source solo-revive eligibility and any prerequisite purchase/interaction needed in the initial area; do not grant revival unconditionally because general perk coverage is deferred.
- [ ] Bind the solo player's required model, skeleton, hit volumes, and essential presentation through the packed content.
- [ ] Convert and bind the source starting pistol, knife, and grenade definitions, inventory ownership, starting ammunition, damage/range, fire timing, reload, melee, throw/fuse, and refill behavior relevant to initial play.
- [ ] Connect required weapon/player/zombie clips and source-timed gameplay notifies; resolve decoder/root-motion/flag cases only as needed by those selected clips.
- [ ] Connect the essential firing, reload, melee, grenade, hit, zombie, barrier, and player-state sound/FX dependencies.
- [ ] Deliver source health/damage, downed/solo-revive/bleedout, death, and restart through the existing systems.
- [ ] Reset player, inventory, enemies, round state, barriers, triggers, animation, audio, and source-owned objects between sessions.
- [ ] Expose usable existing start, pause/resume, restart, and exit flow and make the limited milestone scope clear without presenting deferred content as playable.

## 3. Starting-area survival

- [ ] Convert ordinary enemy definitions, model/animation selection, health, damage, and movement data.
- [ ] Preserve source round population, actor caps, spawn timing, inter-round timing, and health progression.
- [ ] Convert hit-region definitions and source damage-category behavior.
- [ ] Convert barriers, board presentations, repair interactions, and attack/traversal bindings.
- [ ] Bind source zone_start availability, starting-area spawn positions/eligibility, and required initial connection flags.
- [ ] Convert the starting-area navigation/traversal links and connect ordinary zombie pathing, barrier entry, pursuit, attack, hit, and death.
- [ ] Keep unavailable zones and deferred progression out of active spawn/access selection rather than letting unsupported content activate.
- [ ] Preserve point awards, starting barrier repair, weapon ammunition consumption, and round completion through normal gameplay.
- [ ] Bind source HUD data for health/downed state, points, round, weapon/ammunition, and required interaction prompts; keep essential localization/font/image dependencies packed.

### Native client mode and zone contact

- [ ] Recover the source transitions relevant to TriggerClientMode for the supported player states.
- [ ] Update TriggerClientMode from actual player/session lifecycle transitions.
- [ ] Preserve exact source mode exclusions used by zone trigger activation.
- [ ] Connect converted player volumes to zone membership.
- [ ] Preserve source handling for overlapping player volumes.
- [ ] Preserve entry, exit, death/downed, revival, and respawn behavior where the source distinguishes them.
- [ ] Deliver the required source zone events through existing ECS messages.
- [ ] Preserve source single-use activation flag updates and sensor deletion.
- [ ] Clear zone contact/membership state when the player or session is removed.
- [ ] Complete the existing source activation bindings without replacing the committed zone graph.

## 4. Build, open, and prove the initial game is playable

- [ ] Run proportional existing checks for changed pack integration and initial gameplay; keep compile/pack results separate from actual-game outcomes.
- [ ] Build changed compiled inputs with cargo build --features dev-tools --locked and static Bevy linking; use that current executable with the fresh compatible pack.
- [ ] Check all game processes and GPU captures immediately before each launch, wait while occupied, and reuse the owned current executable between scenarios.
- [ ] Open the actual game window and confirm the real Origins scene, starting-area collision, player, weapon, and ordinary zombies load without missing required dependencies.
- [ ] Through normal inputs/messages, validate initial spawn, movement/stances, sprint/dive, mud, starting boundaries, zone membership, and the triggers used by this milestone.
- [ ] Through normal play, validate pistol firing/reload/ammunition, knife damage, grenade throw/fuse/damage, zombie pursuit/attacks/death, barrier entry/repair, and point awards.
- [ ] Validate the source early-round survival loop through round 5, including population/spawn timing, health progression, and inter-round transitions.
- [ ] Validate player damage, downed/solo-revive/bleedout/death, pause/resume, and clean restart without stale enemies, contacts, or held inputs.
- [ ] Confirm essential HUD, prompts, animations, and sound reflect the observed gameplay outcomes.
- [ ] Use the existing explicitly enabled development-only loopback BRP and existing Python/PowerShell scenario tools where applicable; use bounded waits, normal inputs, and reset/release between cases. Call client.shutdown(report) in finally before writing every scenario report and close only the owned game.
- [ ] Report the usable executable/pack location, checks actually run, initial gameplay outcomes, explicit visual approximations, disabled/deferred systems, and any remaining blockers.

The initial milestone is complete only when the current executable opens with the newly generated scoped pack and the survival/session checks above pass. Conversion dispatch, a build, a preview pack, or existing tests alone do not establish playability. Final fidelity and full Origins completion remain in todo_final.md.

## Completed trigger implementation to retain

Primary files: tools/asset_builder/collision.rs, entities.rs, game.rs, scenes.rs, src/environment/brushes.rs, src/environment/triggers.rs, src/environment/support.rs, and existing interaction consumers.

Implementation complete. The installed-source and runtime-schema/Avian collider check passed for all 212 native triggers, 449 convex pieces, 136 mud pieces, and 197 registered player-zone pieces, with zero Phase 2 trigger source gaps. Brush *164 is native entity 3995 (`trig_zone_bunker_4`), preserves both source convex pieces, and activates `activate_zone_bunker_4_tank`. Native repeat/contact filters, damage/use/look consumers, script-created ownership, wait-0 deletion, and session cleanup are implemented. All 25 focused trigger/consumer tests passed, and git diff --check passed. Research findings are saved in the authorized master. Scoped pack/startup and initial-consumer acceptance remain above; full-game coverage remains in todo_final.md. The final static game build was skipped as requested.

### Brush trigger *164

- [x] Resolve the exact installed entity and collision submodel associated with brush trigger *164.
- [x] Recover its source classname, script fields, contents, spawn flags, and activation consumer.
- [x] Preserve its source origin, orientation, local geometry, and world transform.
- [x] Preserve its native convex volumes and triangle shapes where present.
- [x] Represent its actual contact/use/damage behavior using existing Avian geometry and ECS trigger components.
- [x] Bind its source owner and source lifetime instead of leaving an unresolved brush classification.
- [x] Connect the trigger to the exact gameplay consumer that produces its intended outcome.
- [x] Include its required geometry and runtime definition in the existing runtime pack assembly.
- [x] Verify its conversion no longer produces an unresolved geometry, classification, ownership, or dependency failure.

### Remaining native trigger geometry

- [x] Complete required brush-backed trigger_multiple conversion.
- [x] Complete required brush-backed trigger_use conversion.
- [x] Complete required brush-backed trigger_damage conversion.
- [x] Complete required trigger_lookat conversion.
- [x] Complete remaining brush-backed trigger_box and info_volume conversion.
- [x] Complete required non-brush trigger kinds currently reported by game::read.
- [x] Preserve native radius/cylinder geometry without replacing it with a box approximation.
- [x] Preserve rotated box geometry and the committed convex representation.
- [x] Carry required terrain/triangle trigger geometry through the existing collision output and Avian compound path. The installed native triggers contain no triangle pieces; convex and triangle metadata remain supported.
- [x] Preserve source contents and surface flags on triangle and convex geometry.
- [x] Preserve body/brush ownership so dynamic brushes are not duplicated in static collision.
- [x] Resolve source-selected mud volumes and their movement consumer.
- [x] Resolve source player boundary volumes and their zone/boundary consumer.
- [x] Preserve overlap results for multi-volume triggers without double-applying a single source activation.
- [x] Preserve exact scene references for rendered brush geometry.
- [x] Reject genuinely missing or ambiguous source geometry with the specific entity/submodel named in the error.

### Contact, ownership, and script-created lifetime

- [x] Implement source contact category and contents-mask gates for each required trigger consumer.
- [x] Preserve native numeric client-mode predicates where they participate in eligibility.
- [x] Preserve the trigger as recipient and the contacting entity as activator.
- [x] Preserve native repeat behavior without adding an unsupported enter-only or alive-only filter.
- [x] Preserve source wait, single-use, and deletion behavior only where the source actually applies it.
- [x] Preserve queued-contact validity across entity deletion and recreation.
- [x] Implement required deferred event ordering through existing ECS scheduling and messages.
- [x] Ensure a deleted single-use trigger cannot activate again through stale queued contacts.
- [x] Complete script-created trigger spawn, enable, disable, replacement, and destruction as Rust ECS behavior.
- [x] Complete trigger owner cleanup when the parent gameplay object is removed.
- [x] Bind shootable trigger consumers to the correct damage category and source predicate.
- [x] Bind use/look consumers to the correct player ownership and input predicate.
- [x] Reset trigger state and queued activation state between sessions.
- [x] Remove resolved trigger gap entries only after their runtime consumers are implemented.
- [x] Run focused source/geometry checks for changed trigger conversion.
- [x] Keep actual-game behavior assertions open under the applicable initial or final acceptance; they have not been run for this implementation.
