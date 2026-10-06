# Terrain query workspace and moving request construction

Fidelity: INVESTIGATING. Static source qualification supplies an executable aggregation prerequisite within an explicit initialized domain; it does not establish indefinite generation recycling or full scene acceptance.

## Sources and reproducible admission

Read [terrain01](terrain-trace-contract.md), [terrain02 payload](terrain-payload-admission.md), [terrain03 point](terrain-point-triangle.md), [builder0024](0024-original-brush-trace-consumer.md), current trace_shape/terrain reader and their tests. The same admitted licensed disk SHA256 `2aff340f22f0c68bf46511c7bbc6167e4bb84524817f54cae08e8f60e6f9afc3` supplies static instructions. No original code/process was executed/read live.

Bounded CFG decoding follows direct branches, validates function boundaries and hashes complete byte spans using PE mapping. Twenty admitted spans total4708 bytes. External terrain04 qualification-receipt.json contains an object with spans `{rva,length,sha256,calls,jumps,returns}`, independently rehashed against disk. Initial byte-level xrefs are investigative leads only; observed statements below come from admitted bodies, not arbitrary byte matches.

| Complete body RVA | Bytes | SHA256 |
|---|---:|---|
| 2d8770 builder | 1741 | a7cd8754734d8e996c88a1864a0c1be89b818d46692774af2753eed8dd13e9f9 |
| 19a740 TLS getter | 29 | 81ca8d877ba7eb9d7a86463eefe1c2b92e45ce97ca9a8a523708a03a700d596a |
| 481090 owner allocation | 139 | ce9210d86bc54dfeceaf12533225379501ffe6d3c0caa6eafb5688c104eacc72 |
| 481120 map initialization | 366 | ba97eb9691981fd04edb15caae420089e85150171393ed7949a1a4126bc9a315 |
| 2c8050 owner bindings | 104 | f97d7fa3b5fc0fc8db7a0f188018ab8be9d7b66cdfb4fa103bcfcbd87b401ccf |
| 2bcdb0 separate workspace producer | 46 | e65e530fecb5e2f5dde9f7091d61c1f3d0e29c8bf276034e295814dbdf661ff2 |
| 482f10 partition consumer | 216 | 6d831bf847f53cead3bc7077fcbfd2ffa90393d4c88aa5bed15c33d37574b433 |
| 482ff0 moving AABB traversal | 1039 | ea7d1c77b66196add715557b883c5774e4a1fbcafdc50eb384d9db7339301fda |

Additional complete admitted support bodies:1a9780,980e0,bbd0,597be0,159c20,a99e0,1f39b0,2909f0,66c410,67a1a2,236010,93220. Full hashes are in the external receipt; no licensed bytes/dumps or local paths are tracked.

## Workspace ownership and initialization

Getter19a740 reads TLS index at VA33e98e4, FS:[2c] indexed TLS block, its pointer table at block+58, and the requested table entry. Slot3 is the collision workspace record. Setter159c20 writes that same table. Owner binding2c8050 sets slot3 to global record base VA2b018b0 plus24*owner. Owner-table selector980e0 first binds TLS block+58 to VA2850110 plus20*owner, then calls2c8050. Complete93220 passes its context+1c value plus2 to this selector. Main initializerbbd0 binds the base pointer table and owner0. These are native identity anchors, never host-pointer APIs.

Complete481090(owner) sets record.u16[0]=0 and allocates **2*global partition count** bytes, saving pointer at record+4. Count comes from VA2503900, the admitted terrain partition count. A separate array of two-byte elements is allocated at record+8 from u16 globalVA25038c0; its meaning is not needed for point terrain and is not renamed here. Remaining record pointers refer to per-owner copied brush/model/scratch records; ownership04 remains responsible for scene geometry.

Allocationa99e0 requests alignment32 through1f39b0. That allocator adjusts the admitted arena cursor, commits pages when required, and calls66c410(destination,0,requested bytes) before returning. Its scalar byte/dword path writes zero; the separately qualified accelerated tail67a1a2 initializes a zero SIMD vector and writes zero blocks plus tails. Thus stamp storage starts with every word zero, including when existing committed arena pages are reused. Requested bytes are zeroed, not merely assumed zero from virtual-memory allocation.

Map wrapper236010 calls loader481290, then481120. The latter initializes owner0 and1, then owner2 onward for the configured count obtained through2909f0 from globalVA3404704, initializes numeric owner10 explicitly, and calls481090(11). The configured count/value and all actual thread-selection lifetimes are not supplied by these static bodies. Do not invent a maximum owner count or claim nonoverlap of those selected indices without configuration evidence. Allocation count/pointers bind the map state used at this initialization, so aggregation must bind the matching terrain payload/count and retain that ownership until reload/reinitialization.

## Generation and validity domain

Builder2d8770 calls19a740(3), increments the record's first **u16** once per complete query, then copies all24 workspace bytes into its local request. With request base at frame+10, the first qword stored at frame+dc maps generation to request+cc and the pointer high word to request+d0. The other copied pointers are retained separately; they do not make each AABB own a separate generation. Separate producer2bcdb0 performs the same u16 increment then24-byte copy into its destination.

Partition482f10 reads pointer+d0 and generation+cc. Null pointer returns without triangles. Equal stamp at partition ordinal returns. Otherwise it writes that generation **before** dispatching ordered triangles, even if the partition has zero triangles or no contact. Sharing across AABBs/leaves/material owners therefore suppresses later encounters with that same partition in the current query, retaining first-encounter ordering. Native null behavior is not a valid full-map exclusion; missing storage must be an explicit assembly failure.

Observed increment is ordinary wrapping u16 arithmetic. Neither complete increment producer has a zero/wrap check or stamp clear. Fresh initialization gives zero stamps and generation0; query generations1..65535 are distinct from every initial/unvisited stamp. That is a source-qualified valid domain for repeated aggregation without an additional reset assumption, provided queries sharing an owner do not interleave stamp writes. Exclusive mutable workspace access can enforce this safe subset; native concurrent/reentrant scheduling has not been established. A synthetic state derivation confirms ordered duplicate suppression in successive generations. It also shows that an unvisited zero stamp would compare equal at wrap-to-zero **if no intervening reset occurs**. This is a conditional arithmetic consequence, not evidence that an original query exhibits the failure.

**Precise remaining lifecycle links:** qualification has not located an external periodic generation/stamp reset tied to these records, nor arena release/reload synchronization and in-flight query invalidation. The bounded disk xref search found the admitted allocation and increment producers; it does not prove absence of other indirect reset writers. Owner thread selection/init caller scheduling and configured worker count also remain unqualified. An authentic aggregate can receive explicit initialized stamp state/generation or use the fresh1..65535 domain and return a named unresolved-wrap error. It must not choose a new rollover policy, silently clear per leaf, reset after a render frame, or assume a new private generation for each AABB.

## Moving request fields

Source builder uses scalar-f32 center=(hi+lo)*.5, extent=hi-center, and adds center separately to each supplied endpoint. Let centered endpoints S,E, displacement D=E-S. Native request stores:

| Relative field | Source expression/use |
|---|---|
| +0 XYZ | D, the terrain03 point displacement |
| +10 XYZ | (E+S)*.5 midpoint, each axis independently |
| +20 XYZ | D*.5 signed half travel |
| +30 XYZ | bit-cleared absolute signed half travel |
| +40 XYZ | moving axial support(r,r,r+h), populated after the heuristic below |
| +80/+90 XYZ | centered S/E |
| +a0 XYZ | reciprocal1/(S-E), or0 for exact equality via275950 |
| +b8 | supplied mask |
| +bc | point discriminator; nonzero selects1d1230 |
| +bd | computed cross-axis bypass flag |
| +c0/+c4 | radius r=min(extentX,extentZ), half segment h=extentZ-r |
| +cc/+d0 | shared query generation/stamp pointer |

For moving endpoints, +bc is set1 when **(extentY+extentX)+extentZ equals zero**, otherwise0. Under finite nonnegative extents this is precisely the point/capsule distinction. It is not inferred from render transforms. Invalid negative extents whose sum cancels are outside the safe admitted domain. Equal source endpoints select a separate stationary route; that is not terrain03 point aggregation.

The bypass flag is computed before axial-support replacement, using original extents. Travel length L=sqrt((DY*DY+DX*DX)+DZ*DZ). The builder forms swept expanded bounds using r on X/Y and sequential h then r on Z; differences of upper/lower bounds produce widths WX,WY,WZ. Compare the separately rounded products:

`(((extentY*extentX)*extentZ)*16)*L > (WY*WX)*WZ`.

Flag+bd is1 only for strict greater-than; equality gives0. Static constants VA bd3344=.5 and c6db6c=16 are rechecked source bits3f000000/41800000. Finite point extents give left product zero and nonnegative right product, so the flag derives0; this is a qualified consequence rather than a chosen bypass default. Nonpoint extents can legitimately produce either flag; the full capsule branch is not replaced by point-only behavior.

Traversal482ff0 first tests three axial midpoint/absolute-halftravel/extent comparisons. If +bd is nonzero it bypasses the three cross-axis comparisons; otherwise they run. Equality proceeds in every separating-axis comparison. Initial node and descendants use the same request fields. Partition482f10 then selects point1d1230 for +bc!=0, otherwise capsule481940. Native point selection does not supply collision material flags itself; wrapper fraction-reduction writes remain separate.

## Validation and completion limits

Twenty complete bodies independently rehashed; source-bound branches/calls and two constants checked. Independent synthetic stamp-state cases confirm first encounter order, successive generations and the conditional wrap comparison; no native program was invoked. This is documentation/static evidence only, so Cargo gates were not rerun. Rust terrain03 was already focused tested in its prior task.

An ordered point aggregate can now use the exact explicit builder fields and caller-owned initialized query workspace within the admitted validity domain. Indefinite generation recycling, map-unload synchronization, full runtime scene binding and capsule/stationary numerics remain unresolved. There is no full player-world or MATCHED/VERIFIED claim.
