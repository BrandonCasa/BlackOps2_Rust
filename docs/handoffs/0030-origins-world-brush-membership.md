TASK:
Establish the clipmap's static-world brush membership/ownership boundary toward
correctly referenced Rust world-query geometry.

STATUS:
PARTIAL

FINDINGS:
- Source separates map leaves from model-embedded leaves and permits shared/aliased
  ClipInfo providers. Pooled brush storage alone does not establish static ownership.
- Positive, zero and negative node tags are retained separately. Relative offsets,
  adjacent-node handling and root-zero sentinel use remain diagnostic hypotheses.
- The one original read rejects at positive terminal brush_index admission, stage3.
  No graph, membership/overlap counts or original zero-side statistics were obtained.
- No static-world collection is admitted; the terminal value's provider/index
  namespace is the smallest unresolved relationship.

EVIDENCE:
- [Findings0020](../reverse_engineering/0020-origins-world-brush-membership.md): pinned
  T6 declarations/loader, explicitly analogical other-engine source and exchange proposal.
- Only configured `zone/all/zm_tomb.ff`, 78,753,344 bytes, SHA256
  `663c2f6e275fb6a8a2844bcc0e31808abe681ce7c2a2ddc6f616ef98fc7cd0b5`.
- Exit 123 in 1.281s; stages 1-3, WORLD_PREDICATE:brush_index; 430 captured bytes,
  no record. Named failure lacks provider/node/value scalar tuple; culprit unknown.
- Before/after source identities agree; normal signed LIST only, no live game input.

CHANGES:
- Two new owned documentation files; outer agent owns state/index integration.
- Fresh external adapter/fixtures and lossless schema30 proposal; original/native
  dependencies and accepted task0028 artifacts retained unchanged.
- No production Rust, viewer, appearance, damage, mechanics, old handoff or Git change.

VALIDATION:
- Native adapter compile, production link and actual fixture link pass, warnings-as-errors.
- 23 actual adapter cases pass, including shared/submodel/null/range/cycle/signed-tag
  and serialization/retention/work/aggregate-pool capacity cases.
- Nine checker cases pass, including running-union and aggregate terminal-work limits.
- Original meets 120s/2MiB capture/2MiB artifact limits; child/threads cleaned.
- Limited source/tool preservation and documentation checks pass; workspace gates
  skipped because no Rust/Cargo change. Synthetic success does not admit original joins.

FIDELITY STATUS:
INVESTIGATING for membership/access; original collision/gameplay UNKNOWN.

UNRESOLVED:
- Declaring provider/node/list and u16 namespace for rejected terminal value;
  direct indexing into the declaring provider's brush pool remains an assumption.
- Authentic T6 traversal, root/model conventions, transforms and runtime providers.
- Original zero-side validity, static membership, sharing/overlap counts and all masks.
- Decoder allocation extents trusted; independent decoder name-open limitation remains.

RECOMMENDED NEXT WORK:
- A separately bounded failure diagnostic should emit provider/header/pool-sharing
  relation, node/list/terminal ordinal/value and global-pool compatibility before
  rejecting. Resolve the join before world export or Rust
  collection implementation; no automatic retry or broad survey.

SCOPE NOT IMPLEMENTED:
- World query implementation, player hull/movement, collision masks, full map export,
  triangle/terrain/AABB/static-model/dynamic collision, live original comparison,
  engine/capture framework, proprietary tracked data, commits or push. No second read.
