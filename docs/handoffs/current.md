TASK:
Implement catalog-wide native weapon assembly and correct sparse/keyed skeletal conversion, preserving first-person framing and gameplay tuning.

STATUS:
PARTIAL. Implementation and compiler/asset gates pass; complete catalog visual review and actual gameplay regressions remain in progress.

FINDINGS:
58 definitions reference 75 separate attachment models that were omitted. Absent channels overwrote authored rest transforms. Keyed XAnim translations require XModel.trans baselines: native arm baselines are zero, while the Remington pump baseline is [15.514983, 0, -2] inches. OAT glTF locals are instead derived from global baseMat transforms. The installation contains compiled fastfiles/t6zm.exe, without C/C++ game source. Retained decompiled scripts and OAT extraction source were read. Original-game computer observation was stopped at the user's instruction.

EVIDENCE:
External inspection/weapon-assembly contains assembly.json, native-models/* exports/logs, import.json, package.json, staged.json, native-package-hashes.json and actual-window captures. All 129 definitions are accounted: 118 visual, eleven internal/default nonvisual; 141 distinct catalog assemblies (76 view, 63 world, two ADS). The first native survey rendered 240 frames and exposed an empty single-round staging problem. The revised full survey is running.

CHANGES:
Offline composition baking joins/reindexes skeletons, sockets, geometry and inverse binds; preserves variant parts, native offsets, ADS-only assemblies and hidden geometry; rejects missing models/sockets. Native baseline recovery uses the retained OAT exporter plus tools/oat-native-skeleton.patch. Shared conversion preserves absent-channel rest poses and adds recovered native baselines to keyed translation before unit conversion. Native empty/last-shot/rechamber actions share authoritative clocks across rigs and sound timelines. Monkey world animation and Claymore presentation references follow assembled rigs. Content/import contracts validate ADS dependencies, lure world bindings and assembled inverse binds. An opt-in origins.exe catalog survey captures three moving-action stages, ADS, empty/return idle, world views, pause/resume and restart. Main cee19b3 shared clocks and delayed grenade release are retained.

VALIDATION:
Locked Cargo metadata passed. Release workspace builds passed after affected fixes. Formatting and diff whitespace checks passed. Full source/import/package closure validates 2,898 runtime files (1,281,961,822 bytes, 1,478 PNGs); staged validation confirms 2,897 LFS pointers and one ordinary Git game.json. The external executable-relative package matches every validated runtime hash. No tests, fixtures or Clippy were run.
Actual failures: initial preparation exposed default/world socket and placeholder/helper inputs; corrected or explicitly classified. The first packaged load rejected the old Monkey lure binding, now corrected and checked offline. An external junction preview could not be canonicalized by the executable; replaced with a full copied package. First catalog survey stopped on a single-round helper rechamber; controlled staging was corrected. An OAT project-only rebuild lacked SolutionDir; the corrected external build passed. Visual catalog/gameplay acceptance is pending.

UNRESOLVED:
Four native staff melee helper definitions declare crystal attachments on a knife world proxy with no crystal socket; their viewmodels are complete and the native knife proxy is retained explicitly. Slot-eight selection switches at full ADS. Hidden triangles require every positive influence to belong to hidden tag descendants; mixed triangles are retained. Eighteen native auxiliary sound aliases remain unavailable in the existing audio pack and are listed externally. Exact DObj/XAnim evaluation and live original-game comparison remain unverified; native-baseline consumption is a source-grounded reconstruction.

RECOMMENDED NEXT WORK:
Complete and visually review the full catalog survey, then exercise wall weapons, equipment and perk bottle regressions from the final package. Review final candidates and commit locally without pushing.

SCOPE NOT IMPLEMENTED:
New weapon acquisition/gameplay systems, staff mechanics, balance changes, camera reframing, exact original evaluator arithmetic, original live comparison or unrelated renderer/roadmap work. Controlled catalog poses do not qualify unsupported gameplay.
