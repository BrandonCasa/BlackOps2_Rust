TASK:
Complete todo.md per PLAN (4).md; this milestone adds prone, airborne movement, dives, falls and remaining PhD movement effects.

STATUS:
PARTIAL. 344 of 1,219 tasks complete; 875 remain. Eighteen tasks newly ticked. Overall goal remains active.

FINDINGS:
Native PhD script supplies radius 300 inches and damage 1,000–5,000. Stance dimensions, acceleration, air control, jump/dive recovery, fall arithmetic and dive activation threshold reconstruct unrecovered engine behavior as explicit MobilityTuning values. Capsule changes preserve feet, expansion checks penetration, and authoritative eye state supplies camera, aiming, use and throw origins. Fall attribution keeps PhD protection separate from general environmental hazards. The preceding committed prop-lighting hotfix remains retained.

EVIDENCE:
External inspection/solo-content/window-mobility-03/mobility.json passed all fourteen actual-window cases and the process exited. Prone/crouch/standing, 0.3048 m eye, jump rejection/recovery, airborne steering, acceleration 2.6667 to 4.8 m/s, deceleration, diving/landing dip and recovery passed. A 5.0125 m drop delivered 41 damage; PhD rejected the same amount. A diving landing emitted the 7.62 m blast, killed an original German actor through shared attributed damage and protected its owner from 4,870 blast damage. A later 9.5056 m fall delivered fatal damage; normal restart created a fresh player. Prone/dive PNGs were visually inspected. Window-mobility-normal/verification.json then passed normal combat, Round 2, fifteen views and restart.

CHANGES:
Fixed-step Mobility state and Z prone input; acceleration/deceleration, air steering, stance/eye transitions, jump restriction/recovery, dive momentum/recovery, peak/speed landing messages and camera dip. Typed fall impacts, PhD fall protection and prepared native dive blasts extend existing damage/equipment systems. Content/import validation accepts the new modifiers and tuning. One bounded opt-in real-window control workload uses native map clearance and actual systems. Updated existing checklist/state/gameplay documentation only.

VALIDATION:
Locked metadata, final release build, formatting and diff whitespace checks passed. Runtime assembly/final integrity passed: 1,697 files, 1,448 PNGs, 1,064,909,832 bytes. Staged runtime index integrity checked before commit. Initial build encountered a live prior validation executable lock; it completed without interruption. Two driver borrow conflicts were fixed. First live workload used an obstructed spawn-room path/roof; second exposed render-frame one-shot input loss. Selecting a clear native map area and driving immediately before fixed movement produced the final passing workload. Initial preparation import lacked the tools module path; corrected invocation succeeded. Existing nonfatal Vertex_Color logs remain. No unit tests, fixtures, suites, Clippy, delegation, installation or push. Twenty tracked Markdown files.

UNRESOLVED:
Deliberate blocked-expansion qualification remains unchecked, despite naturally observed rejection. Original sprint/dive clips, bob/audio, mud, moving supports and other environmental hazards remain unfinished. Controlled travel, equipment ownership and actor admission qualify branches, not sustained acquisition/survival. Native arithmetic, detailed dive presentation and source equivalence remain unqualified. Earlier complete perk/equipment acceptance workloads were not all repeated.

RECOMMENDED NEXT WORK:
Continue the player/interaction checklist: deliberate blocked stance clearance, walking/sprint bob, source mud volumes/Stamin-Up interaction, hazards and moving support. Then continue dependency-ordered map economy/navigation/quest systems. Keep all remaining tasks and the full goal active.

SCOPE NOT IMPLEMENTED:
Remaining 875 tasks, complete arsenal/special enemies, generators, powered economy, robots/tank/digs, crafting/staffs, quests, full original UI/audio, settings and final solo quest/sustained-survival gates. No second application, replacement engine or original pixel/numeric equivalence claim.
