TASK:
Complete every todo.md task per PLAN (4).md; this milestone implements native map navigation, access gates, cached enemy routes and spawn admission.

STATUS:
PARTIAL. 376 of 1,219 tasks complete; 843 remain. Sixteen tasks newly ticked. The complete goal remains active.

FINDINGS:
Compiled PathData contains 2,776 nodes and 23,197 directed links, including 276 typed negotiation links with native animation identities. Original player volumes yield 200 convex brushes across 98 zones; 338 ordinary/rising spawners retain native activation-zone identity. 206 nodes remain explicitly unassigned/closed. Center-ray floor snapping falsely intersected uphill capsule sides; bounded clearance within the step budget and shared movement-controller edge validation fixed all eleven normal door crossings. Source spawner groups can span physical zones, so activation identity and projected physical membership are separate. Admission follows reverse directed reachability toward the player.

EVIDENCE:
External inspection/solo-content/navigation-recovery/map-navigation.json and navigation-preparation.json retain native extraction and provenance. Window-navigation-06/navigation.json passed all eleven closed/open door gates, active links 507 to 16,225, 38 target replans, 33 blocker replans, four direct pursuits, 1,186 separation applications and a 512-entry cache with 325 hits/162 reachable fallbacks. All door economy/audio/removal and fresh restart cases passed; cache reset to zero and nine starting admissions returned. Window-navigation-normal-02/verification.json passed six Round 1 kills, Round 2, fifteen survey views and restart, with native spawn IDs/positions and no failed population debit. Both final processes terminated. Open-door and original zombie attack PNGs inspected.

CHANGES:
Focused NavigationPlugin, prepared native graph/zone/spawn tables and schema/import validation. Bevy owns scheduling/scenes; Avian owns zone sensors, capsule clearance and shared character motion. Typed zone/door gates, bounded cached routes, target/access replanning, reachable fallback, direct pursuit, local separation and reconstructed zone/offscreen weights. Immutable graph survives restart; mutable access/cache/admissions reset. Existing actual-window driver extended; no tests/suites/fixtures. Door preparation preserves extra access tables.

VALIDATION:
Locked metadata, release workspace build and formatting gates passed. Asset assembly and staged LFS closure passed: 1,708 runtime files, 1,453 PNGs, 1,074,922,110 bytes, 1,707 LFS pointers and ordinary Git game.json. Twenty tracked Markdown files. No Clippy or unit tests. Initial giant JSON macro recursion fixed by separate diagnostic assignments. Two premature builds encountered Windows executable locks; owned stale verification run stopped once, and builds retried after process exit. Index helper initially lacked numpy; rerun with existing uv dependency environment passed. Window-navigation-normal-01 failed with no admitted enemies/pending six; source projection corrected. Navigation-01/-02/-04 exposed rejected stair/village links; bounded capsule support fixed them. Navigation-05 and final -06 passed; normal -02 passed. Existing transient Vertex_Color diagnostics during dynamic assembly remain a rendering limitation, not native equivalence evidence.

UNRESOLVED:
Native negotiation execution and source-volume-unassigned routes remain unfinished. Capsule/step admission, ten-meter source projection and preference weights reconstruct behavior; no original routing equivalence claimed. Stuck detection/recovery exists but final runs did not trigger it, so task remains unchecked. Failed-admission budget/cap branches require their later focused qualification. Physical unopened-zone player admission beyond solid doors, full-map traversal and sustained survival remain unqualified.

RECOMMENDED NEXT WORK:
Implement native enemy climbing/rising and landing/animation synchronization, then barriers and generator defenders/capture. Continue every unchecked checklist task and final quest/survival acceptance.

SCOPE NOT IMPLEMENTED:
Remaining traversal/barriers/generators, special enemies, robots/tank, crafting/staffs/quests, remaining arsenal/audio/HUD/settings and full solo ending/sustained survival. All 843 unchecked tasks remain required.
