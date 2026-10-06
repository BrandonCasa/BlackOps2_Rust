TASK:
Qualify static XModel collision consumers and exhaustive retained provider availability.

STATUS:
COMPLETE

FINDINGS:
- Classic static branch is 1d73d0 -> 486bf0 -> 29acc0 -> 00dfd0;
  initial 2d5a60 and 0ba160 leads instead consume runtime entities and dynents.
- Qualified point transforms, ordered surface/triangle masks, edge tolerance,
  hit writes/ties, sorted instance insertion and sector relocation.
- All 10,837 instances resolve to 420 decoded providers; 510 surfaces / 78,296
  finite triangles. No missing or reference-only provider. 210 contents-zero
  instances have source-backed classic-constructor exclusion only.

EVIDENCE:
- [Detailed contract](../reverse_engineering/static-xmodel-collision-contract.md).
- Static disk PE identity, 19 complete bodies / 8,384 instruction bytes;
  T6 declarations and OAT loader revision 7d027e8.
- Owned external movement-static-model-01 source-only metadata probe and
  recovery receipt; fastfile and prior decoder/producer hashes preserved.

CHANGES:
- Added contract and this handoff only; payload and helper outputs external.
- No Rust reader/query, shared scene contract, manifest or exports changed.

VALIDATION:
- Owned adapter MSVC /W3 /WX compile and link passed.
- Supervised decoder probe passed all instance/provider metadata predicates;
  exact metadata identity 3,209,708 bytes / d322619cd3b97284006f0673e1b3fc817cfaf6a41528b34a1e5752213ee8271b.
- Sixteen independent synthetic f32 point/transform cases passed.
- Documentation link/whitespace checks recorded after writing.
- Workspace Rust gates not run for this documentation-only task; primary owns integration gates.

FIDELITY STATUS:
INVESTIGATING

UNRESOLVED:
- Player motor backend binding and stationary/swept hull static collision.
- Initial sector world bounds/free-list setup and native scene invocation/order.
- Two collmap geometry consumer semantics; full collision payload not emitted.
- No native or original comparison evidence.

RECOMMENDED NEXT WORK:
- Recover/admit shared collision-surface payload with bounded Rust reader.
- Bind qualified player shape consumers to these static providers and qualify
  stationary/hull behavior before implementing the participating family.

SCOPE NOT IMPLEMENTED:
- Rust collision queries, rendering geometry substitution, native execution,
  original capture/play, materials, movement or complete full-map coverage claim.
