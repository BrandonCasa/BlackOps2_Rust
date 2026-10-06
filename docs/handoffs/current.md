TASK:
Commit a performant hotfix for near-black radio cases and similar probe-only props.

STATUS:
COMPLETE for this hotfix. The full Origins reconstruction remains PARTIAL; the last committed checklist has 326 complete and 893 remaining tasks.

FINDINGS:
Radio cases have dark authored albedo and probe-only lighting, while nearby world geometry has baked lighting and gauges have independent emission. Ordinary probe gain 22 underlit these surfaces. Probe-only non-tile props now use gain 60 and a bounded 25-percent floor from their own spherical-mean SH irradiance. Zero-energy probes remain dark. This is reconstructed indirect bounce, not native-equivalence evidence. Gauge emission, atlas/vertex/tile calibration, exposure and existing lamps remain unchanged.

EVIDENCE:
External inspection/prop-lighting-hotfix/final-radio and final-room actual-window receipts passed with zero process exits. Radio fronts/tops, room surfaces and a map view were visually inspected. Final-game/verification.json passed combat, movement, round advancement, 15 survey views and restart. The settled 1920x1080 room sample measured 56.8 FPS (17.59 ms mean), versus baseline 49.0 FPS (20.39 ms); these short eight-second samples do not establish sustained performance. Final close-up measured 84.4 FPS. All 331 shadowed point lights were retained.

CHANGES:
Calibrated the shared probe-only material path. Precompute the spherical-mean fill once per shared material into existing RGB uniform lanes; the fragment adds only a componentwise maximum, without new texture samples, lights or shadow maps. Cache vertex adaptation per provider mesh, count missing assets as pending and release CPU vertices only for completed providers. Added opt-in close-up benchmark views 15/16; the ordinary survey still has 15 views. Updated retained rendering documentation. Unrelated in-progress gameplay edits are excluded from this commit.

VALIDATION:
Locked metadata, release build, format check and hotfix diff whitespace checks passed. Bundled-Python asset integrity passed: 1,697 runtime files, 1,448 PNGs, 1,064,909,635 bytes. No runtime assets changed. Twenty tracked Markdown files. Initial validation used a Python lacking NumPy; switched to the existing bundled runtime. Two initial window attempts exposed CPU mesh release before adaptation; provider caching alone was insufficient, and restricting release to completed providers fixed the final runs. One intermediate cache edit failed compilation because of misplaced patch context and a shadowed mesh identifier; corrected and rebuilt. Final radio/room/game runs exited zero. Existing nonfatal Vertex_Color and extracted RenderMesh logs remain. No unit tests, suites, fixtures, Clippy, delegation, installation or push.

UNRESOLVED:
Native lighting arithmetic, exact lamp-shadow equivalence, every prop/material and sustained frame time/memory behavior are unqualified. Unprepared late mesh CPU copies are deliberately retained; their memory cost was not measured. Separate ongoing gameplay edits were not qualified by this lighting build.

RECOMMENDED NEXT WORK:
Resume the full gameplay checklist. Revisit prop calibration only with concrete problematic views; preserve shared materials, authored lighting and shadow reuse.

SCOPE NOT IMPLEMENTED:
No replacement lighting engine, extra per-prop lights, native pixel matching or broader gameplay milestone. Full Origins, the main quest and extended survival remain unfinished.
