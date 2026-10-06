TASK:
Review and integrate complete external terrain payload admission and the grounded,
collision-access and baseline-parameter source contracts.

STATUS:
COMPLETE

FINDINGS:
- Terrain payload references admit across every declared array and all265 model
  headers. This is data admission, not numerical collision or full-map coverage.
- Grounded movement needs the receiver/contact solver. Classic trace aggregation
  cannot silently replace it. Default parameters and effective receiver creation
  remain explicit source gaps; independently qualified timing/SSE work continues.

EVIDENCE:
- Reviewed [terrain02](movement-terrain-02.md), [native03](movement-native-03.md),
  [native04](movement-native-04.md) and [collision05](movement-collision-05.md), their
  contracts, assets reader/tests and shared export diff.
- Independently rehashed recovered terrain identity and all30 native03/all15
  collision05 bounded disk spans. Licensed payloads and raw outputs stay external.

CHANGES:
- Integrated bounded terrain_snapshot reader/export and six synthetic tests plus
  an explicit external opt-in. No dependencies, lockfile or unsafe policy change.
- Linked reviewed source contracts and replaced current payload/validation status.

VALIDATION:
- Locked offline workspace metadata:11 members. Workspace tests:194 passed,
  zero failed,6 ignored. External terrain opt-in separately:1 passed.
- cargo fmt --all -- --check, git diff --check and locked workspace all-target
  Clippy with -D warnings pass. Logs remain under the ignored inspection root.
- Prior client input/pacing checks and synthetic window GPU smoke remain the
  reviewed host baseline; this assets-only change does not prove a movement client.

FIDELITY STATUS:
INVESTIGATING for source/data access; full controller UNKNOWN.

UNRESOLVED:
- Numerical terrain/world families, receiver/provider/pose/activation scene
  binding, full motor/profile, ECS/replay/client integration and comparisons.

RECOMMENDED NEXT WORK:
- Continue the three owned lanes: timing/SSE implementation, effective receiver
  qualification and static XModel family qualification, then ready family code.

SCOPE NOT IMPLEMENTED:
- Full movement phase, original play/capture/process access, proprietary Git
  content, broad extraction, remote pushes or MATCHED/VERIFIED claims.
