TASK:
Complete todo.md per PLAN (4).md; this milestone implements authored mud, movement presentation and deliberate stance clearance.

STATUS:
PARTIAL. 353 of 1,219 tasks complete; 866 remain. Nine tasks newly ticked. The full goal remains active.

FINDINGS:
The retained map's generic trigger table is empty. Collision submodel *20 contains the authored player_slow_area union: 203 brushes, GUID 8EA6BB4D and native origin (214,812,-25). Recovered GSC defines movement targets 0.6/0.7, decrements 0.1/0.05 and 100 ms sampling; leaving restores one. Avian owns convex sensor queries. Solid masks prevent volumes from blocking movement, targeting, support, navigation or blasts. Upward slope-projected velocity previously broke grounded classification; explicit airborne state now preserves ground contact while retaining jumping. Bob/sway parameters reconstruct unrecovered native presentation.

EVIDENCE:
External inspection/solo-content/mud-triggers-recovery and mud-preparation.json retain the read-only native extraction and provenance. Window-mud-03/mud.json passed eleven live cases: gradual 0.9-to-0.6 slowdown, exit recovery, native Stamin bottle activation and 0.95-to-0.7 slowdown, four mud footsteps, walking/sprint bob peaks 0.018015/0.031069 m, sway peak 0.039526 rad/settling, blocked native expansion, clear expansion and fresh restart after fatal fall. Walking/sway PNGs were visually inspected. Window-mobility-after-mud/mobility.json passed all fourteen prior movement/PhD/actor-kill/restart cases. Final window-mud-normal/verification.json passed movement, combat, Round 2, fifteen views and clear restart; each process terminated.

CHANGES:
Read-only OAT brush-volume callback and external convex preparation; validated environment.mud metadata and source rules. Query-only Avian volumes, shared solid filters, session-owned terrain scale and slope-grounding fix. Scale applies walking/crouch/prone/crawl/sprint/dive travel. Actual ground displacement supplies fixed-step bob and typed surface-selected footstep messages. Viewmodel root sway consumes look changes, preserves authored offsets and decays at rest. Added one opt-in actual-window workload and updated existing state/checklist/gameplay documentation.

VALIDATION:
Locked metadata, final release build, format and diff checks passed. Full assembled runtime integrity passed: 1,697 files, 1,448 PNGs, 1,065,181,673 bytes; staged index integrity checked before commit. Twenty tracked Markdown files. Initial OAT attempts rejected the empty generic trigger table and a zero-content brush node; selecting collision submodels and skipping empty nodes produced the validated union. First window run exposed slope grounding and stale teleport fall peaks. A second workload found no route under overly restrictive clearance requirements; shorter authored routes/overhead checks produced the final passing run. One relocation edit needed a mutable query binding before it compiled. Mud workload passed before final dive scaling/downed eligibility additions; final release and normal gameplay passed afterward, and dive scaling preceded the passing movement regression. Existing nonfatal Vertex_Color logs remain. No unit tests, fixtures, suites, Clippy, delegation, installation or push.

UNRESOLVED:
Original mud audio/VO and native bob/sway arithmetic remain unfinished or unqualified. Mud downed/dive branches, boundary-wide volume matching, every overlapping collider and different-frame-rate presentation were not separately live-qualified. Controlled acquisition/travel proves branches, not normal full-session acquisition or sustained survival. Full perk/equipment acceptance workloads were not all repeated.

RECOMMENDED NEXT WORK:
Continue player/interaction tasks: moving-platform support/displacement/jump detachment, authored environmental hazards/out-of-bounds handling, remaining cursor/input lifecycle and hold/prompt behavior. Continue dependency-ordered navigation, powered economy and Origins progression afterward. Keep the complete checklist/goal intact.

SCOPE NOT IMPLEMENTED:
Remaining 866 tasks: full arsenal/special enemies, generators/powered systems, robots/tank/digs, crafting/staffs, quests, original HUD/audio/settings and final solo ending/extended survival. No additional game executable, replacement engine or original-equivalence claim.
