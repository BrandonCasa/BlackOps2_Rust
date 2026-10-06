TASK:
Complete todo.md per PLAN (4).md; this milestone implements native solo door/debris purchases and zone unlocks.

STATUS:
PARTIAL. 360 of 1,219 tasks complete; 859 remain. Seven tasks newly ticked. The full goal remains active.

FINDINGS:
Native map entities define seven bunker doors and six debris triggers grouped into eleven shared purchases. Solo utility scripts subtract 250 from authored costs, then another 250 only if the reduced cost remains at least 2500. Native blockers move bunker leaves down 120 units (3.048 m) over one second with quarter-duration acceleration/deceleration; Origins deletes leaves afterward. All 160 native adjacency calls are bidirectional by default. The graph contains 98 zones; ordinary door purchases connect 86 from the three initially reachable starting zones. Tank/chamber/crypt flags remain separate. Original pneumatic/stone audio was not recovered; deterministic reconstructed cues provide bounded spatial playback.

EVIDENCE:
External inspection/solo-content/doors-recovery retains native entities, adjacency and decompiled _zm_blockers/_zm_zonemgr sources. Doors-preparation.json records source hashes and 11 groups, 13 triggers, 20 original model leaves and 13 convex clip brushes. Window-doors-03/doors.json passed 24 actual-window cases: all eleven insufficient-funds checks, eleven accepted/held-use purchases with exact solo debits, all associated flags unlocked, all colliders/leaves removed, 20 spatial audio sinks, reachability growth and fresh restart restoring every blocker/leaf with zero purchases and 500 points. Closed/moving/open bunker PNGs were inspected. Window-doors-normal/verification.json passed movement, combat, Round 2, 15 views and clear restart. Both processes terminated.

CHANGES:
Session-owned AccessPlugin/ZoneAccess, native group/prop/brush metadata, atomic door interactions, own-blocker sight exclusions and purchased-group rejection. Bevy owns transformed original scenes, material fill, spatial audio/listener and bounded lifetimes; Avian owns convex solid blockers. Shared offline brush solver reuses existing mud conversion. Import/load gates validate door transforms, convex geometry, references and complete media closure. Runtime game.json preserves its existing CRLF bytes outside changed content. Existing state/gameplay docs updated; no new Markdown files.

VALIDATION:
Locked metadata, release workspace build and formatting check passed. Asset assembly and staged LFS closure passed: 1,708 runtime files, 1,453 PNGs, 1,068,340,531 bytes, 1,707 LFS pointers and ordinary Git game.json. Twenty tracked Markdown files. No unit tests, fixtures, suites or Clippy. Initial compiler failures (field called as method and mutable coverage borrow) corrected. Window-doors-01 failed its narrow church approach search; expanded ring/height sampling fixed it. Window-doors-02 passed every group/audio case but its restart setup landed on the spawn roof without dying; a higher controlled fall fixed the driver. Closed capture timing corrected. Existing transient Vertex_Color pipeline diagnostics appeared during dynamic prop assembly, while inspected prop images and final normal workload rendered successfully.

UNRESOLVED:
Opening sounds and 2.3 m point-target reach reconstruct presentation/use behavior. Manual full-map traversal, original acoustic/particle equivalence and sustained play remain unqualified. Navigation and spawn admission still use the prior room graph; no zone/door traversal-link gates or zone-weighted spawning are claimed. Physical unopened-zone admission beyond these native solid doors remains unfinished. Prior movement/perk/arsenal approximations remain in project/gameplay documentation and Git history.

RECOMMENDED NEXT WORK:
Bind native player zone volumes and map navigation/admission to this access graph, then implement barrier repair and generator capture/defender systems. Continue every remaining todo task and final quest/survival acceptance.

SCOPE NOT IMPLEMENTED:
Zone-controlled navigation/spawning, barriers, generators/local power, tank and quest flags, remaining weapons/enemies/quests/audio/UI and sustained complete solo acceptance. All 859 unchecked tasks remain required.
