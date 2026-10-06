TASK:
Qualify native movement/controller inputs and an Origins unperked baseline from
licensed source and retained evidence, without original game/process access.

STATUS:
PARTIAL

FINDINGS:
- Static Plutonium disk code matches all fifteen retained native collision spans.
- Origins character callbacks supply scale 1, duration 4, cooldown 0; starter c96_zm.
- Five complete native methods qualify player-associated timing/storage/view anchors.
- Ordinary motor, command substeps, stance hulls/masks and numerical rules remain open.

EVIDENCE:
- Project state, architecture, Task0037, physics source/tests and reverse-engineering skill.
- Static licensed executable, retained Task0035 spans and compiled Origins script.
- [Contract and exact next anchors](../reverse_engineering/movement-native-contract.md).
- External movement-native-01 qualification receipt/helpers/raw spans/disassembly.

CHANGES:
- Added movement-native-contract.md and this handoff only.
- Proposed API fields are implementation choices; no shared exports/contract changed.

VALIDATION:
- Five complete functions/internal branch boundaries; fifteen exact retained code matches.
- Compiled script hash and fresh disassembly; four independent conversion examples pass.
- Cargo/native output/live fidelity checks not run: documentation/external-only task.

FIDELITY STATUS:
INVESTIGATING for source configuration; numerical movement UNKNOWN.

UNRESOLVED:
- Actual player movement updater/callback, commands and duration/substep boundary.
- All stance hulls/masks, grounded/air/step/slide rules and c96 movement modifiers.
- Native view interpolation/camera transform and original comparison.

RECOMMENDED NEXT WORK:
- Bounded command-owner investigation from player-associated command storage consumers
  at RVA 0x034bc4/0x034c5c/0x18f205, then motor/trace callback construction.

SCOPE NOT IMPLEMENTED:
- Movement, collision, ECS, window/replay assembly, original execution/capture, commits.
