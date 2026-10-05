TASK:
Obtain and independently replay the bounded ordinary map-leaf brush membership
candidate while preserving the measured separate box association.

STATUS:
COMPLETE

FINDINGS:
- Named diagnostic map-leaf closure selects 8,417 ordinary IDs, reaches 14,551
  nodes, and does not reach tagged box node 14,967/raw value 8,417.
- Positive/zero/negative nodes: 10,872/1,620/2,476; every negative tag is -1.
- 727 map roots are zero; none has nonzero brush contents, 214 have terrain contents.
- All 265 model providers absent; submodel ownership/overlap remains unknown.
- 3,178 selected brushes have zero sides, 6-12 vertices and valid bounds/spans.
  Metadata does not establish axial geometry/source-vertex agreement.
- Ordinary range admission remains strict; box overlap/signature mismatches and
  zero-root brush-content contradictions have explicit failure/unavailable results.

EVIDENCE:
- [Findings0023](../reverse_engineering/0023-origins-map-leaf-brush-membership.md):
  exact counts, limits, source-backed association and diagnostic traversal contract.
- Only configured zone/all/zm_tomb.ff, 78,753,344 bytes, SHA256
  663c2f6e275fb6a8a2844bcc0e31808abe681ce7c2a2ddc6f616ef98fc7cd0b5.
- One original signed LIST read, native 0 in 1.344s; 2,634,532-byte schema33 SHA256
  6b999dba22db62c98a22c469eaad9d4ee7441b7ee10c058405b74f39221e67b3.
- External full IDs/hash, lossless metadata graph, independent summary, sources,
  executables, fixtures and preservation receipts. No geometry bodies.

CHANGES:
- Two owned docs and fresh external task0033 source/fixtures/checker/observer.
- Exact tagged box association preserves raw ID; no out-of-pool brush dereference
  or automatic provider fallback. 8MiB during-write record cap; 2MiB output cap.
- No production Rust, shared native sources, originals, old artifacts or Git mutation.

VALIDATION:
- Compile/production relink/fixture link pass with warnings as errors.
- 39 actual adapter cases, 16 checker tamper/framing rejections, 5 syntax checks pass.
- Original native/replay statuses, reached counts and sorted IDs agree; 40,004 work
  visits and 8,417 retained IDs. Box reached=false; no limits/errors; child/threads cleaned.
- Same-handle input identities and 9 reused/9 qualified files preserved; docs checks
  pass. No retry/owned process. Unchanged Cargo gates skipped.

FIDELITY STATUS:
INVESTIGATING for data interpretation; authentic T6 collision/gameplay UNKNOWN.

UNRESOLVED:
- Authentic T6 branch/root-zero convention and box/runtime/null-provider handling.
- Submodel sharing/transforms; selecting the entire pool diagnostically does not
  prove exclusive world-space static ownership.
- Geometry bodies/source-vertex agreement, zero-side admission, collision masks,
  player hull/movement and other collision families. Decoder allocation extent trusted.

RECOMMENDED NEXT WORK:
- Implement a safe bounded schema33 diagnostic owner collection preserving raw IDs,
  absent owners, interpretation and coverage. Reuse owned brush queries for explicitly
  available geometry; do not claim complete world collision from metadata.
- Treat zero-side geometry/source-vertex validation as an explicit separate admission
  revision. Current upper side/vertex limits need no expansion from measured counts.

SCOPE NOT IMPLEMENTED:
- Rust/world-query implementation, geometry/full asset export, player movement/masks,
  other collision families, consumer/capture framework, live game input, additional
  original observations, proprietary tracked content, commits or push.
