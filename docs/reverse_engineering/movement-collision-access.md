# Movement collision access

Status: INVESTIGATING. Static source qualification establishes caller and result
boundaries, not an executable replacement backend or original comparison.
Read with [grounded movement](movement-grounded-contract.md) and
[global ownership](collision-global-ownership.md). This task owns access producers;
the numerical owner retains main solver arithmetic and step selection.

## Identity and procedure

The licensed disk image SHA256 is
`2aff340f22f0c68bf46511c7bbc6167e4bb84524817f54cae08e8f60e6f9afc3`.
Addresses below are RVAs, except explicitly marked VAs. PE image base is
`00400000`; admitted text raw offset equals RVA minus `c00`. Decode reachable
branches from each entry, stop at returns/tail transfers, save the whole bounded
span, and rehash against disk. External task05 receipt admits 15 spans, 5058 bytes.
No original program or captured function was executed.

| RVA | Bytes | SHA256 |
|---|---:|---|
|12ae20|115|283c060c5103a2ee8b39f4f800ab36f6df8a76f02e30dbc6b9a217c0fe671045|
|27b1e0|884|9b336771ec7b58dbdf7a54beafea539bc960cd09137af4eebc041427b05c6bd5|
|00ff70|773|3c109219223ac071e5cdd402c186bd7e190a2bb9565f34c4c776afefdfd0cf33|
|12bfa0|456|93409de8631e6cd80bfbe85274da54f4b71b92658ebe65d418b2ee72e47a887a|
|103b20|397|7e0d028149e95fda018a2cb59566f8a73f83fbff0780c21692a50a3280c859d0|
|223a30|73|69d638b9a349f6be74ae8787151030100cda74934348caefb97ea20eeaefe588|
|011270|245|5a751644d0e77f722b7e3a63b710ced7643b4758c1bc8141c8fa24fbcf80c977|
|297800|417|ee17144d56492238c398707f6e08df259532797560995480ead90d957efba1c8|
|2c33f0|44|02148c71e8f262ad03e51e9d5e20ff51a781d25baf46f1948c600901adb479ac|
|60c270|727|2a4931e731d2a750a41de6de1e73f5e7484b5cc8c660ec142fcac2be8fa92a7c|
|0855e0|113|ae467271bc45ee0122b63f037397dc4f81ca3a861ef39a5d2e6a63872db9454f|
|231a50|134|926acdee0ca79a6e05379413cba2dc9691794025a3e2fa0ba6df54de3009913c|
|05b0e0|358|b2e89f3a160e2c8e1018afdb614ea9fd08d90a9a12e1bc9b5b3069fe79b34902|
|2d00f0|34|4927f395d0f5d3fec4ea02a4ffe8d4ea54a362f6ad3e60456acf93ef804fdc71|
|162b70|288|4dbd090e248f03ac6dc4a94143e3281d8cf8ae8eecd5bc1b9b520b23ce57ccdb|

## Solver dispatch and separate response schemas

`12ae20(environment, request, output)` clears the word at output+18 and invokes
`2530d0` with selector0 and an explicit inside-byte destination. With no inside
signal it returns that solver's result. Otherwise it sets output byte18=1 and
tries `199760` selectors0 then1. If both fail, byte19=1 and it reruns `2530d0`
with selector1. The inside branch returns integer1. Output XYZ origin at0..8 and
velocity atc..14 belong to the numerical owner's admitted solver contract.
These correction flags are separate from trace bytes2a/2b and ground bytes2c/30.
Both shared main and inside solver bodies call `27b1e0` and `00ff70`; the inside
solver also directly uses `297800` and `12bfa0`.

The clearance caller chain remains `541e10 -> 22ef70 -> 025170 ->
27b1e0/00ff70`. Its already qualified conversion copies contact fraction+30 into
trace fraction+10, copies normal0/4/8, clears byte2a and sets byte2b iff contact
signed scalar+34 is strictly negative. Both positive and negative zero yield
byte2b=0. `541e10` accepts exactly byte2b=0. No equivalence with the accepted
classic stationary brush overlap result has been established.

## Receiver, shapes and candidate storage

`27b1e0` selects cache VA `101e8a0` when environment byte4 is nonzero and
VA `320ebe0` otherwise. `223a30` searches an ordered tree by environment word0;
node+4 is key, +8/+c are children and +0 is receiver. The consumer assumes the
requested receiver exists. It supplies no graceful missing-key result.

Receiver+1e8 is a shape object. Cached bounds at60..74 are compared with incoming
lo/hi, then updated. Shape virtual slot28 supplies a discriminator:4 selects
`307100`,7 selects `3030a0`,5 selects `1ab120`. These numeric discriminators
alone do not qualify shape names. Virtual slot18 computes AABBs using the
transform at VA `320eba0`. Lower bounds add, upper bounds subtract the three
mutable scalars at VA `101e890`. Request+0 receives the shape; +1c the receiver;
+14 receiver+78; +18 receiver+90; +40 origin; +50 end-minus-origin;
+a0 the allocator reached through request+18+74. `011270` carries explicit
environment, mask, owner/filter and transform references into a candidate query.

`00ff70` refreshes that context, then consumes its linked candidates at+a8 and
count+b0. Zero count produces an empty contact list. Scratch allocations hold a
candidate pointer list and aligned 0x50-byte contact records. Candidate+20 is a
shape; candidate+24 is an optional transform. Shape flags+3c bit8 instead choose
shape+34 as transform. Absent transform uses VA `320eba0`.

`297800` dispatches contact generation through `60c270`. The latter uses
`60bf30`, `60c7c0`, `60ce40`, `60cd10`, `60cd70`, `60cbf0` and related support
operations. Their full geometry contracts are outside this bounded task.
Qualified contact fields are normal0/4/8, world point10/14/18, local point20/24/28,
fraction30, signed separation34, virtual query-shape predicate byte38 and
provider3c. Query-shape virtual slots14,28,2c and provider-shape slots14,34 are
also required. Provider selection `2c33f0` may call `2b7160`.

Contacts append in candidate traversal order. Best contact changes for smaller
fraction; on equal zero fractions it changes only for smaller signed separation.
Equal nonzero fractions retain the earlier contact. Fraction bounds are narrowed
with query+c margin and capped at1. When query byte9 is set, a selected negative
separation stops traversal. Candidate gathering/order `018e70`, refresh/filter
helpers and all allocator exhaustion semantics remain unqualified. A Rust
adapter must fail explicitly for unavailable storage or allocation rather than
translate native null dereferences into empty world success.

## Walkability and refinement

`12bfa0` first rejects contact byte38=0. Otherwise normalZ greater than or equal
to f32 bits `3f333333` (approximately0.7) accepts. Lower normals use provider+20
shape virtual slot30. It receives the world contact point and vertical axis
`(0,0,1)`, transformed into provider local space when provider+24 exists.
Therefore the normal threshold alone is insufficient to decide all slopes;
the shape predicate is participating evidence. This function returns a predicate,
not result byte2c/30 writes; those are caller-owned.

`103b20` runs only for query mask bit10 and nonzero global candidate count at
VA `3095cc4`. `21ffa0` gathers at most128 candidate pointers into local storage.
It forwards each candidate's model/pose references to `21a410`. Only a strictly
smaller result fraction updates classification+1c=4 and identifier word+20 from
candidate+14. Equal fractions preserve the existing owner. Candidate ownership,
activation, model and pose sources and `21a410` geometry remain prerequisites;
the ordinary clearance mask includes bit10, so this path cannot be skipped.

## Precise remaining qualification

`231a50 -> 0855e0` initializes the selected cache and query context. Existing keys
reuse their receiver; missing keys call `2d00f0` then `12f820` to insert it.
The admitted disk body of `2d00f0` calls `162b70` on a local record, then returns
zero. `162b70` traverses encoded/global records and other helpers; its complete
runtime effect has not been qualified. This is a disk-code qualification limit,
not evidence that the original backend is empty or unsupported.

Required next evidence: the effective missing-key factory or prepopulation
producer, receiver construction and shape vtable identities, environment mask
and owner/filter binding, candidate refresh/storage linked to clipmap/terrain/
static models/submodels/dynents, their transforms/activation, and refinement
candidate population. Neither nullable serialized ClipInfo nor these unqualified
object bindings excludes participating map geometry. Classic brush access cannot
silently replace these consumers.

All15 spans rehashed against disk;12 independent synthetic boundary cases cover
inside correction flags, threshold equality/next lower f32, eligibility gate,
negative separation and signed zero. They exercise reconstructed predicates only;
no native execution, full backend implementation, original comparison or Rust
behavior test is claimed.
