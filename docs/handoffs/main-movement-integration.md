TASK:
Merge the reviewed full-map movement checkpoint into local main, preserving
main's separately integrated material/rendering work.

STATUS:
COMPLETE (local integration; full-map movement remains incomplete)

FINDINGS:
- Movement checkpoint4dc1055 and main6db0e53 diverged from2837403.
- Asset module exports require both collision and appearance readers.
- Concise project state must retain material progress and movement limits.
- Full-map player movement remains incomplete; this is integration only.

EVIDENCE:
- Reviewed current main/worktree status, source exports/manifests/lockfile,
  viewer merge and movement/material integration handoffs.
- Movement integration06 and main-materials-integration record separate prior
  tests, source investigations, GPU/original comparisons and evidence limits.

CHANGES:
- Non-fast-forward merge preserving both branch histories and implementations.
- Resolved assets exports/project-state conflicts; trimmed four trailing blank
  documentation lines found by the full staged whitespace check.
- No original assets, manifests with local paths, generated builds or secrets added.

VALIDATION:
- Combined locked offline workspace tests:352 passed,0 failed,17 ignored.
- Locked offline all-target workspace Clippy with -D warnings passed.
- Separate boot client-feature tests:8 passed; client all-target Clippy with
  -D warnings passed. These are host checks, not a complete movement replay.
- Locked offline metadata:11 members; workspace formatting and staged whitespace
  passed. Licensed-input/GPU fixtures were not rerun for this merge.

FIDELITY STATUS:
INVESTIGATING; full-map basic movement UNKNOWN.

UNRESOLVED:
- Existing motor/solver, collision coverage, baseline/FP/camera/replay gaps remain.
- Existing materials globals/post-processing/shadow/submodel-placement gaps remain.

RECOMMENDED NEXT WORK:
- Resume bounded movement or appearance tasks under their existing evidence limits.

SCOPE NOT IMPLEMENTED:
- New gameplay/rendering features, original invocation/capture, remote push,
  full movement completion or full original scene fidelity.
