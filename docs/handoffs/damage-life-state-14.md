TASK:
Qualify ordinary actor subtraction/lethal health and test inferred zero-clamp.

STATUS:
PARTIAL

FINDINGS:
- Native direct signed subtraction supported; nonpositive health branch includes
  equality, subject to intervening notification effects.
- No unconditional zero clamp: pre-handler negative health retained/floored-999.
- Final health/death effects depend on unresolved per-type handler.

EVIDENCE:
- Source01 actor wrapper/native10/current Rust; qualified named runtime record,
  complete routine/helper and external task14 receipts. See findings14.

CHANGES:
- Findings/handoff only; external bounded inspector/code receipts.

VALIDATION:
- Named identity/page/hash/repeat and five parser cases pass; final run exit0.
- Reads6087679bytes, retained1239bytes; content/diff checks pass.
- Cargo unchanged; no live comparison or active tool operation remains.

FIDELITY STATUS:
INVESTIGATING

UNRESOLVED:
- Ordinary handler type/pointer and final health/terminal effect; active invocation.

RECOMMENDED NEXT WORK:
- Qualify that single death-handler edge before changing terminal health storage.

SCOPE NOT IMPLEMENTED:
- Gameplay, special actor policy/AI, VM survey, UI/sampling/writes, dependencies,
  nested agents, commits/pushes.
