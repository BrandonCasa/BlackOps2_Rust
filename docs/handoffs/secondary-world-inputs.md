TASK:
Export and safely admit exact hash-bound secondary world corner inputs.

STATUS:
COMPLETE

FINDINGS:
- Every6,469 surface and449,721 corner accounted by source-local indexes, stream offset/format/span and accepted geometry identity.
- All seven formats 0/1/2/3/4/5/7 cover 6,469 surfaces and 449,721 corners; all decode successfully.
- Format3 completed by authorized continuation using actual lit frame27970 event4574/source386: stride8, TEXCOORD3 half4 at0.

EVIDENCE:
- Retained external secondary stream489,788 bytes; native producer corner-local indexes899,442 bytes.
- Original frame6325 layout correlations, format5 frame40264event4692, format7 frame119013event7354.
- External export_secondary.py, secondary-corners.json and secondary-accounting.private.json.
- See ../reverse_engineering/secondary-world-inputs.md for exact layouts and boundary.

CHANGES:
- crates/assets/src/secondary_stream.rs typed loader/decode API and public module.
- Existing internal hash-bound dependency helper shared; v1/v2 schemas unchanged.
- crates/assets/tests/secondary_stream.rs; external native producer emits local indexes.

VALIDATION:
- Focused synthetic plus actual licensed opt-in tests:4PASS.
- Actual complete corner census: 449,721 qualified corners decoded and full strict loader succeeds; zero record bounds failures.
- assets strict all-target Clippy PASS; owned formatting PASS.
- Workspace/rendering/GPU integration remains outer-agent responsibility.

FIDELITY STATUS:
INVESTIGATING

UNRESOLVED:
- Renderer pass-routing integration not implemented in this lane.

RECOMMENDED NEXT WORK:
- Integrate explicit required semantics in original renderer; identity-normal builtin remains separate next task.

SCOPE NOT IMPLEMENTED:
- Builtin images, runtime code-resource generation, viewer/rendering edits, live-game control, new agents, commits or merging.
