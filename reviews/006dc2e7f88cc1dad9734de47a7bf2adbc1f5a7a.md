# 006dc2e7f88cc1dad9734de47a7bf2adbc1f5a7a — Draft tank routes, transactions, platform support and Lightning parts

Static first-pass review of this original revision and its parent. Read every changed Rust file, the committed catalog, moving-platform support, ordinary enemy navigation/damage integration, and recovered tank scripts/entities. No build, tests, game, or repository scripts were run. Draft line numbers refer to this original revision. Reference paths are relative to `inspection/`.

## Findings

### P1 — The moving tank's exact presentation ID is absent from the committed catalog

- Draft: `src/tank.rs:37-44`.
- Reference: `game-prepare/entities/maps/mp/zm_tomb.d3dbsp.ents:2640-2645`, `:3095-3104` associate the original vehicle with model `veh_t6_dlc_mkiv_tank`; `todo-audit-20261010/decompiled/t6/zm_tomb_tank.gsc:245-281` configures that tank as the moving platform.
- Trigger: start a session or activate the tank.
- Current behavior: creates the platform/collider, then conditionally looks for the exact ID `actor.veh_t6_dlc_mkiv_tank`. The committed `assets/origins/game.json` has no presentation with that ID and no presentation ID containing `tank` at this revision. This commit changes no catalog/asset entry to supply it, so the conditional child `WorldAssetRoot` is never inserted.
- Reference behavior: a visible tank model moves along the path and supplies boarding/weapon tags.
- Impact: this plugin moves an invisible solid platform, leaving the player's primary boarding and route reference absent. A static tank already baked into a map, if any, would not become this entity's moving visual.
- Correction direction: register the original prepared tank scene with an exact explicit catalog mapping and load it on the moving entity; verify its dependency closure and transform/tag basis when asset work is authorized.
- Tip status: the reviewed initial tip `8cf043721` still has no matching presentation and keeps the same lookup.

### P2 — The remote call box is usable before the tank has ever been activated

- Draft: `src/tank.rs:119`, `:133-139`.
- Reference: `todo-audit-20261010/decompiled/t6/zm_tomb_tank.gsc:251`, `:432-443` (`tank_station`), `:549-581` (`tank_call_boxes_update`).
- Trigger: reach the station opposite the tank's initial church position and use its call box before buying the initial tank activation.
- Current behavior: every call box whose site differs from `tank.station` is immediately eligible; paying 500 starts a remote trip and grants a free onward trip.
- Reference behavior: initializes `tank_boxes_enabled` to zero and only enables remote boxes after the first tank-use trigger. Hidden/unavailable boxes cannot substitute for that first physical activation.
- Impact: the initial route/progression requirement and timing of access to the remote tank service change.
- Correction direction: retain the source first-activation flag and include it in remote-box availability, while preserving the source called-trip/free-following-trip behavior after unlock.
- Tip status: still present.

### P2 — Every paid/free activation adds a four-second stationary delay that the source does not have

- Draft: `src/tank.rs:65-68`, `:136-137`; initial route speed at `src/tank/source.rs:2-3`.
- Reference: `todo-audit-20261010/decompiled/t6/zm_tomb_tank.gsc:597-610` immediately sets tank speed to 8 mph on activation; `:704-717` waits 0.4 seconds only for engine-loop audio. `:446-448` waits four seconds in the independent left-behind VO check.
- Trigger: activate the tank from its body or use the free trip after calling it.
- Current behavior: holds position for four seconds in `Departing`, then starts traveling the first segment at the next node's stored speed (3 mph on the initial leg).
- Reference behavior: begins movement immediately at the explicitly set 8 mph; the nearby four-second wait belongs to a VO/left-behind observation thread, not departure movement.
- Impact: boarding windows, travel timing, and Lightning-piece jumps shift on every trip. The first departure is additionally slower than the source command.
- Correction direction: separate audio/VO timers from movement activation; begin departure with the recovered speed command and then apply node/path speed semantics at the corresponding path events.
- Tip status: still present.

### P1 — Recent damage immunity can cancel a tank runover

- Draft: `src/tank.rs:86-90`; integration at `src/damage.rs:222-225` in this revision.
- Reference: `todo-audit-20261010/decompiled/t6/zm_tomb_tank.gsc:664-668` (`tank_ran_me_over`) explicitly calls `disableinvulnerability` before applying health plus 1000 damage.
- Trigger: the player enters the front runover region during the normal post-hit immunity window.
- Current behavior: emits an ordinary `ImpactSource::Vehicle` crush for `health.maximum()`. The central damage resolver rejects every player impact except `OutOfBounds` while `Recovery.immunity > 0`, including this vehicle impact.
- Reference behavior: runover intentionally disables invulnerability and downs/kills regardless of that protection window.
- Impact: taking a recent hit can let the player survive a tank runover that the original makes unavoidable.
- Correction direction: represent the source runover protection override explicitly in the shared damage/life pipeline, with proper attribution, rather than relying only on a large damage amount.
- Tip status: still present.

### P2 — A downed runover victim is left in the tank's collision/route instead of moved to a safe nearby node

- Draft: `src/tank.rs:86-91`.
- Reference: `todo-audit-20261010/decompiled/t6/zm_tomb_tank.gsc:664-703` selects a nearby valid path node, reserves it for the downed player, fades the view, moves/links the player there, and then unlinks.
- Trigger: a tank runover causes a human to enter last stand.
- Current behavior: applies crush damage but has no victim relocation, safe-node selection, or reservation. The actor loop then excludes `Downed` actors, so the resulting victim receives no further tank handling.
- Reference behavior: relocates the downed player clear of the tank to a nearby valid map position as part of the runover outcome.
- Impact: the player can remain under/inside the moving or parked solid and have a different escape/revive position; a successful self-revive can begin in an obstructed location.
- Correction direction: connect a reusable runover outcome handler to the normal downed transition, select/reserve a valid nearby navigation position, and complete a bounded relocation/restoration sequence.
- Tip status: still present.

### P2 — Riding the tank does not activate the source zombie chase/boarding behavior

- Draft: `src/tank.rs:37-39`, `:77-91`; platform integration changes only `src/movement/platforms.rs:31` visibility.
- Reference: `todo-audit-20261010/decompiled/t6/zm_tomb_tank.gsc:275-277`, `:1105-1227` (`tank_zombie_think`) and its tag/climb transitions. Moving-tank pursuers can use `super_sprint`, claim boarding tags, climb, and continue attacking on the tank.
- Trigger: an ordinary zombie targets a passenger on the moving or parked tank.
- Current behavior: the new entity supports player carrying and crushes actors in front, but adds no tank-specific enemy goal/tag claim, climb traversal, onboard support, or pursuit gait. Existing `src/enemies.rs` navigation still targets the player through the fixed map graph and solid tank body.
- Reference behavior: actively changes pursuing zombies into tank chase/tag chase/climbing/on-tank states so riding does not bypass the normal threat.
- Impact: the main passenger combat loop is absent; obstacles and height can keep normal ground navigation from reaching the passenger instead of producing source boarding attacks.
- Correction direction: add tank-aware goals and traversal through the existing enemy/navigation system, using the original tank boarding/jump-down tags and normal damage path. Preserve ordinary pathfinding when a zombie leaves the tank state.
- Tip status: still present. This is a confirmed missing integration, not a runtime assertion that every passenger position is invulnerable.

### P2 — Tank flame damage is replaced by a front-only crush box

- Draft: `src/tank.rs:86-91`.
- Reference: `todo-audit-20261010/decompiled/t6/zm_tomb_tank.gsc:1413-1553` (`tank_run_flamethrowers`, target selection, cycle, and `flamethrower_damage_zombies`).
- Trigger: zombies approach the moving tank's front/side flame target regions.
- Current behavior: only a local front-side position check emits `DamageCategory::Crush` for the actor's maximum health. There are no original flame tags, target cycles, periodic damage, or flame states.
- Reference behavior: operates three tag-based flamethrowers with bounded flame/pause cycles and their dedicated target/damage rules. The source's player runover trigger is separate from this zombie damage.
- Impact: enemies disappear by instantaneous crush in a different region, while side/forward flame interactions and the associated combat timing are unavailable.
- Correction direction: retain player runover as its own trigger/outcome, and implement source flame targets/cadence through the reusable emitter/damage path using the original tag transforms.
- Tip status: still present.

### P2 — The recovered route loses the tank's conditional powerup-drop events

- Draft: `src/tank.rs:93-105`; `src/tank/source.rs:1-38` stores only position, speed, and stop.
- Reference: `todo-audit-20261010/decompiled/t6/zm_tomb_tank.gsc:85-129` (`tank_drop_powerups`) initializes three drop nodes, requires riders and all Lightning staff pieces picked up, selects a supported powerup, and schedules the node again after 8–11 rounds; node/entity tags are at `game-prepare/entities/maps/mp/zm_tomb.d3dbsp.ents:2589`, `:2864`, `:2923`, `:3056-3070`.
- Trigger: ride past an eligible drop region after acquiring all Lightning parts and reaching that node's next eligible round.
- Current behavior: reaching a node only increments the route index or enters cooling. The route contains no preserved event tag or per-node next-drop-round state, and movement never requests a powerup drop.
- Reference behavior: produces the route's repeatable conditional bonus drop and updates its round cooldown.
- Impact: later tank rides lose a source gameplay reward after the initial staff-part acquisition.
- Correction direction: retain original node identities/event tags and add rider/part/round-gated drop scheduling through the existing powerup producer, with the recovered pool and per-node round schedule.
- Tip status: still present.

## Boundaries

The 500-point ordinary price, called-trip free onward ride, two stops, and rider-count-based cooldown clamped to 2–120 seconds are represented. The `PartState::Consumed` change correctly prevents already crafted ice parts from respawning; see the prior crafting report for that historical defect. Collider dimensions, source path interpolation/acceleration/steering, exact platform carrying, and the physical reachability of Lightning pickup jumps remain unvalidated because this pass intentionally does not run the game or compiler. No runtime or release readiness claim is made.

