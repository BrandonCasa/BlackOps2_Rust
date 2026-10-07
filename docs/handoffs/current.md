TASK:
Fix missing Vertex_Color material specialization errors and missing blood/overlay opacity in one final local commit.

STATUS:
COMPLETE for this bounded rendering hotfix. The larger goal remains PAUSED at the user's request; 415 of 1,219 tasks are checked, 804 remain.

FINDINGS:
The custom forward adapter requests RGBA vertex colors unconditionally, while glTF colors are optional. Pack audit found 385 uncolored primitives across 116 files; ordinary standard-material paths already tolerate these. The layered surface shader treated overlay alpha only as a reveal threshold, allowing fractional opacity to saturate at fully painted vertices. All twelve blood-material combinations use that shared path; their original textures contain fractional alpha (blood-smear maximum 215/255), not missing texture data.

EVIDENCE:
External inspection/solo-content/window-vertex-colors-01/verification.json passed the vertex-color fix with zero Vertex_Color errors. Window-vertex-colors-alpha-02/verification.json passed the final combined build: six kills, Round 2, fifteen rendered map views and restart; stdout/stderr contain zero ERROR entries. Settled playing.png was inspected against the earlier view: wall blood and floor stains retain soft opacity instead of solid patches. The initial spawn screenshot caught shader warm-up and was not used as appearance evidence. Both game processes exited.

CHANGES:
Mesh preparation inserts white RGBA only when ATTRIBUTE_COLOR is absent, preserving authored vertex colors and alpha. Both reveal-controlled overlay coverages multiply texture alpha before blend/multiply/add albedo composition and normal/specular response mixing. The fourth overlay already uses texture-alpha coverage. All 164 layered materials, including twelve blood combinations, share the corrected path. Base walls stay opaque; prepared textures and runtime asset bytes are unchanged.

VALIDATION:
Locked metadata, release workspace build, fmt check and diff review passed. The actual final game window compiled its shaders and completed the normal driver with zero logged errors. Staged runtime integrity still passes 1,734 files, 1,733 LFS pointers and ordinary Git game.json. No unit tests, suites, fixtures or Clippy. Twenty tracked Markdown files. One final local commit; no push.

UNRESOLVED:
Native reveal parameters, source radiance and unrelated actor cutout appearance remain approximations or unfinished. Appearance inspection covered settled representative stains and the existing fifteen-view survey; it does not claim pixel equivalence or an exhaustive manual inspection of every decal placement.

RECOMMENDED NEXT WORK:
Remain paused until the user resumes the larger implementation goal.

SCOPE NOT IMPLEMENTED:
No additional gameplay TODOs, shader warm-up/loading redesign, native pixel matching or unrelated rendering repairs. The 804 unchecked tasks remain required when resumed.
