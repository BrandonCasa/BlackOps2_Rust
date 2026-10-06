TASK:
Implement and export the measured original identity-normal builtin with exact source resolution.

STATUS:
COMPLETE

FINDINGS:
- Original live initializer/common/upload helpers prove one linear RGBA8 pixel[128,128,255,128],1x1,one mip.
- All41identity-normal material references now resolve a shared image260; original260image indexes and samplers preserved.
- Shadow builtin's single reference remains explicitly unresolved.

EVIDENCE:
- Capture worker relocated actual live name/function table and read-only initializer/upload bytes.
- Public DXGI format28 definition; external export_builtin.py and builtin-definitions.private.json.
- See ../reverse_engineering/identity-normal-builtin.md for source/evidence/DDS hashes.

CHANGES:
- crates/assets/src/builtin_images.rs exact typed definition and external-byte validator; public module.
- crates/assets/tests/builtin_images.rs synthetic and actual full-scene join checks.
- Existing optional actual scene test accepts compatible260/261image tables.
- External scene+bank now261images,identity-normal41bindings resolved; bank schema unchanged.

VALIDATION:
- Synthetic plus actual scene/bank tests2PASS; all41sampler/semantic/hash joins checked.
- Exact format/one mip/pixel data and changed-identity/pixel/mip rejection checked.
- assets strict all-target Clippy PASS; owned formatting PASS.
- Workspace/GPU integration remains outer-agent responsibility.

FIDELITY STATUS:
INVESTIGATING

UNRESOLVED:
- Shadow builtin generation and full runtime code resources are outside this bounded task.
- Renderer must retain linear interpretation for this normal input.

RECOMMENDED NEXT WORK:
- Integrate the expanded image table and retain explicit shadow/code-resource errors.
- Qualify format3 secondary layout separately.

SCOPE NOT IMPLEMENTED:
- Shadow builtin, runtime resource generators, viewer/rendering/game edits, game control, new agents, commits or merging.
