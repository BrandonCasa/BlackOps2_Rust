TASK:
Qualify the native basic first-person camera boundary for Bevy integration.

STATUS:
COMPLETE

FINDINGS:
- Source-backed pitch/yaw/roll degrees and forward/right/up basis permit a supplied-eye adapter using existing coordinate conversion.
- Basic command angles convert signed integers by 360/65536, center and clamp pitch through two unresolved runtime providers; no mouse sensitivity or pitch default is invented.
- Eye uses interpolated current height, then bob, right-axis displacement, lean and a minimum-Z clamp. Base eye alone is insufficient for general basic movement.
- Projection and active render-view ownership remain explicit unresolved inputs.

EVIDENCE:
- [Camera contract](../reverse_engineering/first-person-camera-contract.md), command/stance contracts, existing raw input/gray host and coordinate adapter; local Bevy0.19.1 transform/projection source.
- Licensed disk SHA-256 2aff340f22f0c68bf46511c7bbc6167e4bb84524817f54cae08e8f60e6f9afc3; 19 bounded bodies, 1,213 instructions, 4,469 bytes; final receipt and all instructions remain external.
- Script registration references ground eye/angle leads. One guessed FOV decode entry is explicitly rejected; no camera role is assigned from its output.

CHANGES:
- Camera contract and this handoff only; no shared code, exports, manifests, state, defaults or external original files changed.

VALIDATION:
- 20 independent synthetic source-equation checks passed; disk before/after hashes and exact instruction mapping passed.
- Document links and git diff whitespace checks passed. No Rust code changed; workspace/build gates belong to primary integration.
- No original comparison, original invocation, process inspection, window run or capture performed.

FIDELITY STATUS:
INVESTIGATING

UNRESOLVED:
- Pitch provider values/registration, bob accessor/vector values and state/time producers, lean producer/domain, special eye paths, final rendered-view publication.
- Mouse-to-command encoding/scaling, projection/FOV/aspect/near/far ownership, baseline Origins camera/startup pose and native x87 trigonometric bit reproduction.

RECOMMENDED NEXT WORK:
- Qualify the missing basic runtime camera parameters and final view/projection consumers before assembling an authentic camera; implement the bounded supplied-eye/basis adapter after review.

SCOPE NOT IMPLEMENTED:
- Rust camera/motor/command changes, any guessed camera defaults, special camera modes, rendered view effects or full-map/controller completion.
