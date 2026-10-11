# Pass 2: 006dc2e7f88cc1dad9734de47a7bf2adbc1f5a7a

Draft tank routes, transactions, platform support and Lightning parts

Corresponding pass 1 replacement: `5db9a32665e71e57c7473c37cd25e6387725cbc5`; prior report: `reviews/006dc2e7f88cc1dad9734de47a7bf2adbc1f5a7a.md` (preserved verbatim).

## Review basis

The exclusive base is `adc228ac458148236cb93755550388df1ac3f2a8`; draft endpoint is frozen at `5f4673a49c2a9c04437ff5c433468971a97ab15b`, and pass 1 endpoint at `ebe7eefcc9e7afa60a6484b066b53776bceb057c`. Locations below refer to the original reviewed commit unless stated otherwise. The complete commit diff, its prior report, necessary committed callers/consumers and later changes through the frozen endpoint were read. Dirty files were excluded. This report changes no implementation and makes no runtime-validation claim.

Research references are relative to the canonical `t6zm-master` research root; asset references prefixed `inspection/` are relative to the recovered inspection root. Read `README.md`, `identity.json`, `source/provenance.json`, `state/verification.json`, `state/query-verification.json` and `evidence/unresolved.json`. Native identity: packed SHA256 `f6f7104af2bd0c2b931ea1e739969e85845e499affbf686144c6b490b344ab1e`, unpacked SHA256 `ebf8b363455c22ed25a323a3ba0a45318cc0dd09769d4ec0ba3fbbac7d01251c`; Ghidra 12.1.4 receipts. Existing callback bindings and cross references were queried in `research.sqlite3` read-only/immutable; exports were not regenerated. The committed runtime catalog is Git blob `29747327f454e9ad66212b355f3914bedcb03e65` (`assets/origins/game.json`, SHA256 `5f9063fdb93216baeebfc98b22f20f8d9467a0b3ab38aa8b83363cabff55df60`), unchanged across this portion and the endpoint. No asset from another checkout was substituted.

Changed-path coverage:

```text
M	src/buildables.rs
M	src/crafting.rs
M	src/environment/parts.rs
M	src/lib.rs
M	src/movement/platforms.rs
A	src/tank.rs
A	src/tank/source.rs
M	todo.md
```

## Pass 1 dispositions

| Prior finding | Disposition and endpoint status |
| --- | --- |
| 1. Missing tank presentation ID | Revised/resolved later: exact `actor.tank` lookup is absent at this commit, but original tank scenes do exist under source-model IDs. `394aa5a84891e45aa37484effbb3337ce5ef69d2` selects the native scene. |
| 2. Call before first activation | Confirmed; current. |
| 3. Four-second departure/speed | Confirmed; current. |
| 4. Runover blocked by recovery | Confirmed; current. |
| 5. No downed safe-node relocation | Confirmed; current. |
| 6. No tank chasing/boarding | Confirmed; current. |
| 7. Missing flame attacks | Confirmed; current. |
| 8. Missing route powerup events | Confirmed; current. |

## Native and asset evidence

Recovered `inspection/game-prepare/entities/maps/mp/zm_tomb.d3dbsp.ents:2640-2646,3095-3104` identifies `veh_t6_dlc_mkiv_tank` and the authored route/station. The committed catalog contains `source.model.637309077daedb839c8181d5.lod0`, scene `library/models/zone/all/zm_tomb/637309077daedb839c8181d5/veh_t6_dlc_mkiv_tank_lod0.gltf#Scene0`. Thus the historical failure at `src/tank.rs:37-44` is a bad exact ID, not a missing preserved tank asset. The later native-scene selector prefers the actual zm_tomb path over a patch-zone variant; no different pack was substituted in this review.

Native `disableinvulnerability` at `005eb2c0` clears the player-state invulnerability bit (`exports/functions/005e/005eb2c0.c:14-23`). Source `tank_ran_me_over` deliberately calls this before damage; it is not a cosmetic script name. Native `dodamage` at `00626c60` passes damage and attribution to `005f0dc0` (`exports/functions/0062/00626c60.c:25-105`, instruction `00626f44`), confirming actual health damage. Native `linkto` binding `0061d630` delegates to `0061d3f0`; exact linked-body collision replication is not assumed.

## Actionable correctness findings

### P1  -  Tank crush must bypass recent-hit immunity

Trigger: touch the moving crush region immediately after another injury. `src/tank.rs:86-90` queues an ordinary damage request; `src/damage.rs:222-225` drops it during Recovery immunity. Source `source/server-tomb/maps/mp/zm_tomb_tank.gsc:664-668` disables invulnerability and applies current health plus 1000. Draft makes an otherwise lethal runover harmless.

Correction: add a narrow forced-crush/bypass-recovery policy to the existing damage request and resolve it through normal downing, shield/helmet exclusions and ownership rules; do not globally disable recovery. Confidence high; current at endpoint.

### P2  -  Gate remote calls until the first activation

Trigger: use a remote call box before any tank purchase. `src/tank.rs:119,133-139` accepts without the source first-activation flag. `zm_tomb_tank.gsc:251,432-443,549-581` initializes/updates call boxes only after the first station activation. Draft changes initial access and economy.

Correction: persist `activated_once` in the existing Tank resource, set only on successful station activation and gate remote box eligibility/feedback. Confidence high; current.

### P2  -  Remove the invented departure wait and recover source segment speeds

Trigger: buy/call an idle tank. `src/tank.rs:65-68,136-137` adds four stationary seconds, then follows a route beginning at 3 mph (`src/tank/source.rs:2-3`). Source `zm_tomb_tank.gsc:597-610` starts at 8 mph immediately; `:704-717` delays engine-loop audio only 0.4 seconds; `:446-448` uses four seconds for an independent left-behind VO check. Draft shifts boarding/Lightning jump windows and travel time.

Correction: start motion at accepted activation with source route segment speed changes; run VO/audio timers independently. Confidence high; current. A Bevy fixed-step vehicle/platform component can preserve observable timing without reproducing native vehicle internals.

### P2  -  Relocate downed runover victims safely

Trigger: tank damage downs the player along the route. `src/tank.rs:86-91` leaves the body in the crush/platform region. Source `zm_tomb_tank.gsc:664-703` chooses/reserves a valid nearby path node, fades and moves/links the downed player, then unlinks. Draft can leave the victim obstructing or repeatedly intersecting the tank.

Correction: after the damage outcome, request one safe relocation using recovered route/path candidates, ground/clearance queries and a reservation until completed; preserve downed state and normal life ordering. Confidence high; current.

### P2  -  Activate source tank chase/boarding enemy behavior

Trigger: ride the moving tank while zombies pursue. `src/tank.rs:37-39,77-91` only supplies a platform/crush box; no tank-specific pursuit/boarding state exists. `zm_tomb_tank.gsc:275-277,1105-1227` upgrades moving-tank pursuit to super sprint, claims boarding tags, climbs and continues attacking on board. Riding is substantially safer and enemy position/attack timing differ.

Correction: add tank goal/boarding components to ordinary Zombie AI, reserve native boarding slots and transition through existing movement/animation/attack systems. Use the existing moving-platform support after boarding; do not build a second physics engine. Confidence high; current.

### P2  -  Restore the conditional tank flamethrower cycle

Trigger: zombies enter the tank's side flame zones. `src/tank.rs:86-91` offers only a front crush region. Source `zm_tomb_tank.gsc:1413-1553` selects flame targets, cycles emitters and damages zombies while preserving source player immunity (`zm_tomb.gsc:403-421`). Draft omits combat and feedback around the vehicle.

Correction: add flame-cycle timers/target queries to Tank state, use original tag/zone positions, normal damage messages and imported FX/audio; retain crush as a separate cause. Confidence high; current.

### P2  -  Preserve route node powerup events

Trigger: reach a drop node with riders after all Lightning pieces have been collected and its round cooldown has elapsed. `src/tank/source.rs:1-38` retains only positions/speeds/stops; `src/tank.rs:93-105` never dispatches drop events. `zm_tomb_tank.gsc:85-129` defines three eligible nodes and re-arms each after 8-11 rounds; original entity tags are in `inspection/game-prepare/entities/maps/mp/zm_tomb.d3dbsp.ents:2589,2864,2923,3056-3070`.

Correction: annotate the existing route node table with source event identities and per-node cooldown state, sample only on eligible crossing, and emit ordinary powerup spawn requests. Confidence high; current.

## Historical repair and coverage limits

The missing exact tank scene was a P1 historical visual failure; later native-scene binding repairs that root at HEAD. Dynamic animation/turrets, authored road clearance and static duplicate replacement still require separate assessment and are not certified by a model lookup. Read the entire tank/platform/crafting-consumed-state diff, route data, call/capture transitions and source damage/boarding/flame/drop callers. `PartState::Consumed` fixes the prior crafting Ice uniqueness issue and is not a new defect here. No asset import, runtime platform experiment, compile or test was performed; native inferred linked-body prototypes and unresolved research functions were not treated as authoritative collision semantics.
