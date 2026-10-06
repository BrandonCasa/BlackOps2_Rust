TASK:
Complete all 1,219 todo.md tasks according to the accepted complete solo Origins plan.

STATUS:
PARTIAL - 58 tasks completed; 1,161 remain. Goal active. First content-contract batch committed as bd1c1a5.

FINDINGS:
Version-two typed catalogs/references are available; most remain unpopulated. Read-only patch-resolved recovery exports 129 weapons, 13 attachments, 56 attachment variants, 4,670 entities, 2,220 navigation nodes and 1,713 native animation files. Authored GUIDs repeat; hashed field content plus occurrence disambiguates entity IDs. Seven original skeletal actors are prepared with meter-scale positions/translations/inverse binds and normalized skin weights. Original files remain untouched.

EVIDENCE:
External inspection/solo-content/window-v2/verification.json passed combat, movement, Round 2, restart and 15 rendered views. window-actor-pack/verification.json passed map rendering/restart with expanded definitions; it does not render actors. runtime-skeletal-content.json records 1,292 runtime files, 1,275 PNGs and 751,674,520 bytes. Seven actor glTF closures independently validated (40 dependencies). Global joint transforms multiplied by converted inverse binds have maximum identity error 0.000101307. recovered-gameplay.json retains source fields/hashes and animation headers.

CHANGES:
Typed content/error/index contract, media/skin importer, Bevy features and runtime LFS allowances. Reproducible --recover-gameplay-content and --prepare-actors preparation paths; preserve expanded catalogs during regeneration. Extend existing OAT callback to recover actor images from explicit original image banks. Import Dempsey/Nikolai/Richtofen/Takeo arms, Mauser viewmodel and German zombie body/head through LFS. No actor gameplay binding yet.

VALIDATION:
Source extraction and actor preparation passed, including repeat execution. Complete asset import passed. Locked metadata/build/formatting gates passed for both batches; actual-window outcomes recorded above. Staged expanded closure passed (1,291 LFS pointers plus game.json); Git LFS fsck passed. Initial indentation, missing DDS inputs, duplicate GUID assumptions, Python version, relative index-check path and C++ required-variable placement failed then were corrected. Direct OAT image export returned zero while logging errors; recovery now checks logs and uses the successful existing callback/image-bank path. No tests, fixtures, Clippy, replacement toolchain, delegation or push.

UNRESOLVED:
Native animation files are exported but not decoded into glTF clips. Prepared actors are not yet animated or rendered in gameplay; temporary pistol/zombies remain. Native weapon fields are external intermediates and not yet populated into runtime weapon behavior. Map placement ID rollout and trigger/brush/traversal conversion remain unchecked. Full quest/survival/device/relocated-package gates pending.

RECOMMENDED NEXT WORK:
Decode version-19 native tracks using the retained OAT CompiledXAnimLoader/Writer source; convert compatible local bone tracks into actor glTF clips and notification content, then bind original first-person/enemy presentation. OAT already converts model geometry/global root rotations into Y-up; do not apply that basis twice. Preserve explicit fixed gameplay ownership when replacing temporary systems.

SCOPE NOT IMPLEMENTED:
The 1,161 unchecked tasks and final playthrough gates. Prepared skeletons are asset-integrity evidence, not animated gameplay or original equivalence.
