TASK:
Complete every todo.md task per PLAN (4).md; this milestone implements six generator initial captures and prevents player entry into zombie-only repair pockets.

STATUS:
PARTIAL. 415 of 1,219 tasks complete; 804 remain. Twenty-three generator tasks newly ticked. The full goal remains active.

FINDINGS:
Native solo capture costs 200, takes 12 seconds inside, decays over 20 seconds outside, awards 100 and refunds the purchaser after one second if still eligible. Native contested means capture in progress; guards do not halt its clock. Board pivots lie inside exterior pockets: a player clip at the pivot still permits crossing the sandbags. The permanent clip now sits at the native interior landing point, independently of board state, and only player movement queries its layer. Repair targeting moves toward the playable side of the reconstructed use volume.

EVIDENCE:
External inspection/solo-content/generators-preparation.json and generators-import-final.json retain source/closure evidence. Window-generators-06/generators.json passed 29 cases: insufficient funds, held-use latch, outside decay/resume, complete abandonment, six completions/refunds, 28 guards admitted and cleared, zero guard admission failures, 13 spatial sinks, original mechanical/fluid playback and restart. Window-player-ingress-04/barriers.json passed nine center/edge walk, crouch and sprint-jump attempts; each stopped 2.492 meters inward from the board pivot. Zombies crossed broken boards, all repair/cap/cleanup cases passed, 36 sounds played and restart restored state. The blocked-window PNG was captured during the movement attempt and inspected. Window-generators-normal-02/verification.json passed six kills, Round 2, fifteen rendered views and restart; five ordinary admissions were rejected without consuming budget, with 24 boards destroyed and five entries recorded. Final processes exited.

CHANGES:
Typed session generator capture states, atomic interaction/economy, rewards/refunds, original six pump rigs/eight clips, separate mechanical/fluid playback, alpha preparation and spatial audio. Independent defender population and native emergence projections feed generic enemy position goals; existing navigation/combat/corpse ownership remains shared. Generator HUD status/progress. Permanent layer-four player clips at all twelve authored repair entrances; player capsule sweeps, jumping, stepping and stance clearance include them, while actor/navigation/projectile/use queries do not. Bounded actual-window driver retains the safe endpoint for repairs and approaches with melee when ammunition is exhausted.

VALIDATION:
Locked metadata, release workspace build, fmt check and diff check passed. Imported runtime closure: 1,734 files, 1,461 PNGs, 1,109,833,834 bytes. Staged integrity passed 1,733 LFS pointers and ordinary Git game.json. No unit tests, suites, fixtures or Clippy. Twenty tracked Markdown files. Initial driver macro errors corrected. Generator -01 through -04 exposed standing/use targeting failures; near-side use targeting fixed them; -06 qualified abandonment and alpha. Normal -01 timed out after four kills after chasing into the exterior pocket. Ingress -01 allowed the pocket before its pivot clip; -02/-03 rejected crossing but their repair ground probes left the driver out of range. Landing-point clips and repairs from the actual movement endpoint passed -04; ordinary -02 passed.

UNRESOLVED:
Player clip extents, repair/generator use volumes, vat alpha, spatial mechanical audio and crusader capture defenders are practical reconstructions. All twelve boundaries assemble, but only the starting window's center/edges were exercised. Full generator appearance, original defender model/VO, outer attack slots and full-map boundary coverage are unqualified. Existing Vertex_Color pipeline errors and actor cutout appearance persist. Generator attackers, shutdown progress, native power consumers, Pack-a-Punch and initial/item rewards remain unfinished.

RECOMMENDED NEXT WORK:
Implement native generator attack/shutdown/recapture waves, local power consumers and associated rewards. Continue every remaining atomic task and the complete legitimate solo quest/sustained survival acceptance.

SCOPE NOT IMPLEMENTED:
All 804 unchecked tasks remain required, including the full arsenal, drops/crafting/staffs/quests, special enemies, robots/tank, remaining presentation/settings and complete solo ending/sustained survival.
