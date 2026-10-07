# Current handoff

TASK:
Continue the active complete-TODO goal sequentially. Establish the release baseline, then complete the approved Powerups and Mystery Box milestone with all native weapon prerequisites.

STATUS: PARTIAL

FINDINGS:
The latest user instruction stopped all three agents and banned delegation. Their interrupted worktrees are preserved. None of their gameplay drafts has been accepted, built, validated or integrated. Main starts at fd8911784. Research reads local decompiled scripts, native executable disassembly and weapon exchanges. No original-game executable was launched.

The approved first milestone covers 58 TODOs, followed by the rest of todo.md. Dependency order:
1. Nine weapon prerequisites: complete native box pool and view/world assets, burst/shell/dual-wield actions and ammunition, Ray Gun/Mark II/War Machine projectiles, Monkey tactical acquisition. Extend existing inventory, damage, explosion, rig and audio systems.
2. Nine lifecycle tasks: session reset, source eligibility/probability, earned-score threshold, four-per-round limit, spawn, expiration, pickup radius, duplicate rejection and timed effects. Use fixed-step state and bounded presentation caches.
3. Seven core effects: Insta-Kill damage/scoring, Double Points awards/restoration, Max Ammo reserve/equipment replenishment and delayed Nuke with its one final points award. Keep earned scores distinct from purchases/refunds.
4. Seven Zombie Blood tasks: target suppression, duration/repeat/early exit, visual/disguise presentation, hidden-entity reveal/hiding and targeting restoration. Missing quest entities remain separate TODOs.
5. Fifteen box tasks: initial site, purchase, source roll eligibility/ownership/quotas, open/cycle/take/swap/expiry, teddy/refund/relocation, moving and beam presentation. Reuse interaction and transaction ownership.
6. Three Fire Sale tasks: valid temporary sites, ten-point pricing and restoration, preserving outstanding rewards and native relocation waits.
7. Three powerup presentation tasks: announcements, expiration flashing and original timed HUD presentation.
8. Five integrations: round drops, barrier points, generator rewards/locks, equipment refills and clean restart. Qualify real gameplay, then repeat matched release measurements.

Source review: Origins filters included powerups before regular admission; Carpenter is absent. Native randomint(100) <= 2 or a pending earned-score threshold admits at most four drops per round inside authored player volumes. Threshold starts at 2,500 and its increment grows by 1.14. Expiration totals 26.5 seconds (15 plus fifteen 500 ms, ten 250 ms and fifteen 100 ms flashes). Pickup uses native origin distance.

Nuke snapshots eligible AI after 500 ms, skips explicit immunity/marked/shield/override branches, waits 100-700 ms before each actor and awards 400 multiplied points after the sequence. The draft's population-only eligibility needs typed source exemptions. Further _zm_spawner.gsc review proves nuked actors can still drop powerups after wait_network_frame; the draft incorrectly excludes all Nuke deaths. Tactical-grenade kills and explicit no_powerups flags reject drops. Native death/drop origins use ground traces. Nuke skips ordinary death-point awards; its damage call is unattributed, requiring explicit kill-stat ownership review. Blood suppresses player targeting for thirty seconds and ends on downing/game end; its flat red draft overlay does not qualify the native model/FX.

Box selection shuffles the catalog and accepts the first eligible candidate. Ownership/upgrades, limited quotas, DLC eligibility and ray-gun exclusion participate; Mark II's gate is checked when encountered. Tomb adds a cycling wait and twelve-second reward lifetime. Generator capture controls local locks. Teddy uses 15/30/50 percent bands and an eight-pull first-location guarantee. Relocation follows the initially shuffled site order and reshuffles on wrap. Fire Sale repeats extend thirty seconds. Drafts still need complete authored sites/pool, models/animations and source callbacks. Dual Five-Sevens require independent trigger, magazine and reload ownership; the left-native weapon has separate Left animation fields that the existing right-action importer does not bind. Preserve resolved runtime stock semantics instead of treating raw magazine-count fields as round counts.

EVIDENCE:
External root: E:/Coding/Captures/origins-powerup-box-20261007/. baseline-package/runs preserve six exploratory runs: moving median 18.6745 FPS / 64.2597 ms p95; combat 220.4567-841.9438 ms p95 with varying populations. These are not acceptance baselines. baseline-cohort-package/runs preserve an attempt stopped after finding enabled render diagnostics. Failed/exploratory receipts are retained.

baseline-shipping-package/runs first removed profiling and stabilized combat. Its moving samples still encountered first-use streaming stalls and lost fixed ticks, so those results remain exploratory. baseline-qualified-package/runs is the final workload revision 3 baseline. Moving warms two complete camera sweeps without body movement, waits for no outstanding texture reads, then begins the same wall-clock camera/movement path each run. Combat admits six production actors before its ten-second warmup; player health is 10,000 and actor health 1,000,000 only for this recorded workload. Measurement includes every frame over thirty wall-clock seconds and the complete final frame; no slow frames are filtered or skipped.

All final runs use the standalone release binary, adjacent validated pack, physical 1920x1080, RTX 4080 Vulkan, SMAA, GPU culling, no VSync and no Tracy/Chrome/render diagnostics or per-frame pipeline inspection. They retain live AI/combat/animation/audio, HUD and the production renderer. Raw intervals, p95/p99/FPS, fixed ticks, counts, streaming/lighting/audio, RAM and identities are external. Executable SHA256: cd2307a29dcf39e4f0c80253f165ae74ec37046cea06895284c73e53e89757c3. Asset-manifest SHA256: 2bdc05d11b6bf5516f18d46a7c2fe1753692037b901b1e74ace729b19df90da0.

| Workload | Run FPS | Run p95 milliseconds | Median FPS | Median p95 milliseconds |
| --- | --- | --- | --- | --- |
| Moving | 20.7798 / 19.3866 / 19.9252 | 60.8654 / 64.6257 / 79.1133 | 19.9252 | 64.6257 |
| Combat | 8.9776 / 9.7124 / 10.2891 | 155.8757 / 132.8710 / 139.5998 | 9.7124 | 139.5998 |

Each final run spans 30.012-30.075 wall seconds, with 1,801-1,805 fixed ticks. All end with zero streaming errors/pending reads/detail. Every combat run starts with six actors and 100 shots, ends with the same six actors and 313 shots, zero kills and exactly 3,130 damage-awarded points. Each measured sample adds 213 shots and 2,130 points. Moving warmup spans 43.49-45.08 seconds. These matched workloads still have run-to-run p95 variability; a small median change requires careful confirmation. The agreed post-integration gate compares three-run medians: p95 <= 103 percent and FPS >= 97 percent of baseline. No post-feature regression result exists yet.

Read _zm_powerups.gsc (threshold, admission, selection, grab/timeout, Nuke/Max Ammo/timed effects), _zm_spawner.gsc (zombie_can_drop_powerups, delayed/grounded drops, zombie_death_animscript and kill attribution), _zm_score.gsc (award/earned-score ownership), _zm_magicbox.gsc (purchase, eligibility, teddy, reward, relocation), _zm_magicbox_tomb.gsc (authored presentation/timing), _zm_powerup_zombie_blood.gsc, zm_tomb.gsc (included catalogs), zm_tomb_capture_zones.gsc (locks), retained weapon fields and executable disassembly. Originals remain untouched.

CHANGES:
Opt-in release benchmark arguments, a fixed-step input driver/wall-clock sampler and tools/benchmark_release.ps1. Ordinary launches do not allocate or run the driver. The runner uses the standalone executable and adjacent pack, hashes identity, preserves failures, shares one game slot and only stops its own process on collision/timeout. No gameplay draft or TODO completion is included yet.

Preserved interrupted drafts: E:/Coding/Worktrees/origins-powerups-20261007, origins-mystery-box-20261007 and origins-weapon-prerequisites-20261007. Do not resume agents/delegate. Reconcile these incomplete edits sequentially.

VALIDATION:
Locked metadata, release build and format check passed; the final build took 59.70 seconds. Runner parsing and diff whitespace checks passed. Fresh external assembly validated all 4,139 files / 3,301,685,797 bytes; baseline-qualified-asset-closure.json retains every file hash. Earlier tools/build.ps1 packaging failed on a preexisting obsolete release schema; that directory was preserved and fresh external packaging used. All six final real-window runs passed with package-relative discovery and no profiler enabled. The game slot is free. No unit tests, suites, fixtures or Clippy. This qualifies the controlled benchmark, not survival/progression or original gameplay equivalence.

UNRESOLVED:
Complete all native gameplay/content/presentation drafts. The projectile draft loses residual movement after a bounce and needs collision/penetration review. Baseline p95 varies across runs; investigate small or borderline performance changes with matched confirmation. Whole-game play and original equivalence are not established by compile/import checks.

RECOMMENDED NEXT WORK:
Complete native weapon prerequisites first, prepare/validate powerup and box assets, correct the source mismatches above, integrate state/effects with the existing application, and exercise real gameplay. Repeat tools/benchmark_release.ps1 with Baseline pointing to baseline-qualified-runs; investigate regressions exceeding three percent. Keep the immutable baseline package/receipts. Continue every remaining TODO afterward.

SCOPE NOT IMPLEMENTED:
All 58 gameplay tasks and later TODOs remain open. No original-game launch, quality reduction, synthetic tests, fixtures, new engine/viewer, push or delegation. Prior renderer work remains in Git history.
