# BO2 movement gaps

46 behavior gaps and unresolved parity questions from static review of the Rust recreation and the BO2 research master. Descriptions deliberately leave the implementation open. **Known** identifies an observed difference or absent path; **Open** identifies behavior whose exact parity is unresolved, including techniques already partly supported.

Close these by inspecting `C:\Users\brand\Documents\T6ZMResearch\t6zm-master\`, following native callers, script consumers, selected data, and installed-version conditions. Public demonstrations identify techniques to investigate; the master determines the actual behavior. Preserve native quirks and unintended techniques where applicable. A generic mechanic existing does not establish that its speedrunner applications work. Do not assume BO1, BO3, multiplayer, or modded behavior applies to BO2 Zombies.

## Sprint and weapon transitions

1. **Known — Sprint-jump stamina accounting.** The native 800 ms sprint-end adjustment and the distinction between jumping from an active sprint and canceling before jumping.
2. **Known — Cancel-before-jump ordering.** The difference between an earlier sprint cancellation and fire/reload/jump inputs arriving together.
3. **Known — Sprint recharge and exhaustion transitions.** Native interruption, recovery, cooldown, and restart-threshold behavior beyond the current stamina counter.
4. **Known — Sprint release and restart rules.** Held sprint, release requirements, jump-related resets, and recovery from interrupted or exhausted sprinting.
5. **Known — Weapon-dependent sprint endurance.** Native weapon and perk contributions to available sprint time.
6. **Known — Sprint weapon-state timing.** Native raise, loop, exit, and readiness transitions that determine when sprinting or firing can resume.
7. **Open — Action cancellation chains.** Reload, ADS, melee, weapon switching, and double-switch inputs interacting with sprint and retained momentum.
8. **Open — Effective directional speeds.** Zombies-selected forward, backward, sideways, diagonal, and sprint-strafe behavior rather than registration defaults alone.

## Strafing, commands, and jumping

9. **Open — Ground-strafe gain.** Native speed gains, steering angles, and alternating-strafe behavior across supported inputs.
10. **Open — Air-strafe gain and momentum retention.** Continued turning, held or released directional inputs, and changes of desired direction during a jump.
11. **Open — Wall strafing.** The native speed advantage and momentum response while moving against walls at different angles.
12. **Known — Frame-rate-sensitive movement.** Native command timing and rounding behind frame-rate-dependent jump height, airtime, glide, and gap reach.
13. **Known — Input sampling boundaries.** Native command history, button edges, directional precision, and mouse/input ordering versus the current frame-to-fixed-tick handoff.
14. **Known — Jump repetition timing.** Native jump eligibility measured from takeoff versus the recreation's recovery measured from landing.
15. **Open — Chained walking hops.** Landing, friction, recovery, and renewed takeoff during the walking hops used between sprint strafes.
16. **Open — Crouch strafes.** Midair stance changes, body clearance, feet placement, and momentum through low openings.
17. **Open — Head contacts.** Ceiling, doorway, and overhang impacts during ordinary jumps and crouch jumps.
18. **Open — Jump trajectory limits.** Native jump-height limiting, vertical integration, and departures from an unobstructed flat-ground jump.

## Collision, slopes, and stance

19. **Known — Slope velocity projection.** Native conditional handling of ascending, descending, and already-airborne motion across ground normals.
20. **Open — Wall and corner clipping.** Momentum through glancing contacts, intersecting planes, narrow passages, and repeated contacts within one movement command.
21. **Open — Airborne step and lip clearance.** Native treatment of steps and small ledges approached during a jump.
22. **Open — Stair and edge transitions.** Step selection, step-down behavior, speed loss, and the boundary between supported and falling movement.
23. **Open — Tiny ledges and perches.** Native support and landing behavior on corners, pegs, narrow edges, and partial footprints.
24. **Known — Player collision bounds.** Native stance-dependent dimensions and contact footprint versus the recreation's capsule geometry.
25. **Known — Standing through movement inputs.** Native stance changes requested through jump or sprint, including blocked transitions.
26. **Known — Prone orientation and clearance.** Native turning, support, slope, and obstruction rules beyond changing body height.
27. **Open — Dive admission.** Sprint history, stance, ground contact, equipment state, and competing inputs at dive initiation.
28. **Open — Dive trajectory.** Launch, apex, ceiling contacts, gravity, and transitions into falling or landing.
29. **Open — Dive slide and exit.** Steering, friction, surface dependence, ledge interactions, slide duration, and recovery into another action.
30. **Open — Repeated dives.** Native exhaustion, recovery, and re-entry timing across consecutive attempts.
31. **Open — Slick and steep surfaces.** Source-selected sliding, acceleration, friction, speed limits, and possible momentum redirection; do not presume patched-out bounce behavior exists.
32. **Known — Mantle traversal.** Native eligibility, surface selection, clearance, movement, and exit behavior where enabled for Zombies.
33. **Known — Ladder traversal.** Native attachment, climbing, turning, dismount, and jump-off behavior where applicable.

## Weapon-assisted and contact-assisted movement

34. **Known — Firing boosts.** Player displacement and momentum from Boomhilda and Ray Gun Mark II firing, including airborne and view-direction-dependent applications.
35. **Known — Melee lunges.** Native target acquisition and movement during a knife or other supported melee charge.
36. **Known — Lunge cancellation and launch behavior.** Jumping, switching weapons, losing the target, and installed-version restrictions during a lunge.
37. **Open — Damage and actor contact.** Native push, body blocking, impact-related movement restrictions, and escape timing around zombies or other actors.

## Mud, platforms, and Origins shortcuts

38. **Known — Mud coverage.** Source-selected slow-area geometry and activation versus incomplete runtime volume conversion.
39. **Open — Mud hopping.** Contact boundaries, entry and exit timing, airborne movement, and preserved momentum across mud and dry surfaces.
40. **Open — Combined movement modifiers.** Stamin-Up, held weapons, ADS, stance, and scripted slowdowns interacting during traversal.
41. **Open — Moving-platform momentum.** Carry, rotation, takeoff, detachment, reattachment, and inherited movement from the tank or other native movers.
42. **Known — Tank contact and stance.** Native deck and hull clearance, passenger detection, prone restrictions, and movement around the tank's edges.
43. **Open — Origins tank-gap and church shortcuts.** The reach and collision conditions behind demonstrated ordinary and weapon-assisted shortcut jumps.
44. **Open — Lightning-staff shortcuts.** Tank dismount/reboarding, boosted approaches, tiny-ledge alternatives, and access to the relevant staff-part platforms.
45. **Open — Shortcut-dependent boundaries.** Door and zone state, invisible collision, death barriers, and rescue behavior encountered when bypassing intended routes.
46. **Open — Robot-foot traversal.** Movement, collision, entry timing, and exit/ejection behavior around native robot interactions.

## Research starting points

Paths below are relative to the research master, except the Rust column. These are entry points, not required designs or exhaustive proofs.

| Area | Research master entry points | Current Rust entry points |
| --- | --- | --- |
| Sprint accounting and ordering | `exports/functions/008c/008c41a0.c`, `008c2560.c`, `008c28e0.c`; `exports/functions/008b/008be3e0.c`, `008be4a0.c`, `008bb920.c` | `src/movement.rs`, `src/player.rs`, `src/weapons.rs` |
| Movement commands and strafing | `exports/functions/008c/008c7de0.c`, `008c6de0.c`, `008c6170.c`, `008c64b0.c`; `exports/functions/008b/008bea40.c`, `008be7e0.c`, `008bebf0.c`, `008bef20.c` | `src/lib.rs`, `src/input.rs`, `src/movement/physics.rs` |
| Jump, slope, and contact behavior | `exports/functions/008b/008b3c10.c`, `008b3920.c`, `008b3540.c`, `008be2b0.c`; collision callees reached from `exports/functions/008c/008c64b0.c` | `src/player.rs`, `src/movement/physics.rs` |
| Dive and traversal | `exports/functions/008a/008ad170.c` and its dvar consumers; `exports/functions/008b/008b3d10.c`, `008b4a90.c`, `008b4520.c`, `008b3a40.c`; movement dispatch in `exports/functions/008c/008c6de0.c` | `src/movement.rs`, `src/player.rs`, `src/movement/physics.rs` |
| Weapons, boosts, and lunges | Native weapon-definition and firing callers in `exports/functions/`; `evidence/weapon-loader-20261012/`; `exports/functions/008c/008c1d50.c`, `008c1e50.c`, `008c20d0.c`; `exports/functions/008b/008b64d0.c` and selected dvar consumers | `src/weapons.rs`, `src/gameplay.rs`, `src/projectiles.rs`, `src/origins/content.rs` |
| Origins scripts and geometry | `source/server-tomb/maps/mp/zm_tomb.gsc`, `zm_tomb_utility.gsc`, `zm_tomb_tank.gsc`, `zm_tomb_giant_robot.gsc`, `zm_tomb_craftables.gsc`; native collision/trigger consumers and recovered map references | `src/environment.rs`, `src/environment/brushes.rs`, `src/movement/platforms.rs`, `src/tank.rs`, `src/robots.rs`, `tools/asset_builder/game.rs` |

## Public technique leads

These demonstrations informed the behavioral questions above. Their explanations, numerical advice, and patch applicability still require confirmation in the master.

- [J Dubs: sprint-cancel strafe-jump discovery](https://www.youtube.com/watch?v=yLVs4vHtujM&t=100s) — cancellation before jumping and airborne stamina recovery.
- [Ultimate Black Ops 2 Zombies Movement Guide](https://www.youtube.com/watch?v=3H-6SLAGAYA) — ground strafing, air strafing, wall strafing, crouch strafes, walking hops, frame-rate effects, weapon speeds, and firing boosts. Transcript read.
- [yOJURT: Black Ops 2 Origins Speedrun Tricks Tutorial](https://www.youtube.com/watch?v=_scEDQXp9VM) — tank-gap and church approaches, Boomhilda boosts, lightning-part shortcuts, narrow landings, and boundary interactions. Transcript read.
- [Black Ops 2: FPS versus jump-height demonstration](https://www.youtube.com/watch?v=Q0vdNSByMvQ) — additional frame-rate sensitivity lead; numerical results not adopted as recovered tuning.
- [BO2 Zombies knife-lunge tutorial](https://www.youtube.com/watch?v=8FInUnqmGvQ) — historical technique lead; installed-version applicability remains a source question.

No gameplay implementation, build, or runtime verification was performed for this list.
