# Movement collision receiver producer

Status: INVESTIGATING / PARTIAL. This bounded inquiry follows the exact missing
receiver producer from [access task05](movement-collision-access.md). It qualifies
the static dispatch machinery and records the effective-body gap. It does not
prove an empty backend or replace the receiver with classic brush access.

## Licensed inputs and reproducible checks

The admitted disk SHA256 is
`2aff340f22f0c68bf46511c7bbc6167e4bb84524817f54cae08e8f60e6f9afc3`.
Use PE section bounds to read metadata VAs, subtract image base `00400000` to
obtain RVAs, and map through the corresponding section raw offset. Code entries
below remain in text, where raw offset is RVA minus `c00`. Decode every reachable
direct branch from each entry, retain complete bounded spans outside the repo,
and rehash those bytes independently. Indirect calls stay explicit dependencies.

| RVA | Bytes | SHA256 |
|---|---:|---|
|142fa0|34|2ca7229a6a4624bc467771fb054f80c019a5d9bb628c960789524b26dc4f5f69|
|0e83f0|5|a792b7556da9466b2cd0da7680b69db34d0afdd0cd99023c23f1f2165e73ba15|
|5a5c10|452|938f4d489b1ae869e852e9cc51d2e365e18d5a6a2b279c45f1dfb8732933f4d2|
|0afec0|55|293cd701238a0e3654023185626e6d8f866cdd9814627bac87ffbe654c8aa7b7|
|027db0|7|f797427556d75398c1f9a083a30969db0573f8406f00fff64e38cbf16a983c63|
|11a7d0|194|7f326345dc75ff481e4c60b8ae8f09615800eb8fcdbb64882e0dc31fd526c6f3|
|269880|95|7c886175796c12cbdc14a769a2193703f17470c62c7d19a1ba54d3c4022fa274|
|0c0160|314|608c3e3dc528367db296e5339d49e19ab69b4b2164535716b8eaba35a485628b|
|044a30|283|686324f240c99affb9a284bb3ae75b57593d6cfd9accaec54b7978bc2dcbe7bc|
|014ad0|3|19d200194d49b34530548575f37a1bb4203ee60f18d42f0084e86b7a8c899faa|
|082be0|89|cb7cb5aba8ea99b5b0f39f11e6c0b1e85ef8ee7f35c9267f7507a166c743b54e|
|1d77a0|268|0b9a1f96400a3d87d7a982235a359f54525f43222a4baeb0d1e14e04d09adf10|

The task06 external receipt covers 12 spans / 1,799 bytes. Five independent
structural checks cover wrapping relative-address resolution, factory return
offset, file-header identity and relocation-normalization examples. These are
static validation, not execution of the original or proof of receiver behavior.

## Observed effective dispatch chain

`231a50 -> 0855e0` selects the cache described in task05. Missing keys call
`2d00f0`, which calls `162b70` with a local record and then has a zero return in
its admitted disk body. The helper has significant indirect effects, so its
ordinary return value alone cannot establish the effective factory behavior.

`162b70` initializes a checksum-like object through `142fa0`. The object starts
with state `ffffffff`; `1d77a0` builds its lookup table from the mutable polynomial
at VA `33dae20`, with bit reflection enabled. Byte updates in `0afec0` use the
low eight bits of byte XOR state, shift state by eight, and XOR the selected
table entry. `027db0` complements the final state.

The helper derives an image base from the relative word at VA `c08cd8`. It
subtracts that base from its caller return address. For this factory, the return
VA is `6d0104`, hence offset `2d0104` when the base is `400000`. It processes
102 metadata records (offsets zero through `194`, inclusive, stride four), then
appends that four-byte caller offset. `5a5c10` handles referenced byte ranges and
relocation information; relocation type3 entries subtract the difference between
the actual image base and the expected base before feeding those words to the
checksum. This establishes why copying an unqualified range/hash from another
image is insufficient to derive the effective callback.

`11a7d0` transforms the final checksum through `082be0` and XOR `07070707`, then
uses that key to decode relative pointer words. It builds another local object
and calls `269880`. The latter installs a structured exception frame and invokes
an indirect function pointer supplied as its second argument, forwarding eight
arguments. That callback pointer is the derived image base, not a directly
identified receiver constructor. Subsequent `0c0160` / `044a30` branches modify
global encoded state; their interpretation as licensing, protection or loader
behavior remains a hypothesis, not an admitted gameplay abstraction.

## Precise disk/effective-body mismatch

In the admitted disk, VA `c08cd8` contains `ff7f7328`. Wrapping addition resolves
it to `00400000`. The expected base at VA `c38bf8` is also `00400000`. That address
is the file's PE image header, beginning with the MZ signature. No independently
qualified effective receiver-producing body is bound there by the admitted disk
metadata. A runtime patch, decoded body, altered metadata or exception-mediated
effect has not been observed or reproduced in this task. None may be assumed.

The exact missing evidence is the effective callback at the derived base and its
eight-argument contract, including any code/metadata transformation that makes it
produce the receiver; the mutable checksum polynomial and encoded-state inputs
needed to select its request; and the resulting receiver allocation, vtable and
shape initialization. Without those, the cache insertion path cannot be source
qualified as a producer of a usable nonnull receiver. The existing-key path still
requires a qualified prepopulation producer. A cache lookup is not evidence of
which collision families populate its candidate context.

## Other licensed variant and retained evidence

A second local licensed executable has SHA256
`f6f7104af2bd0c2b931ea1e739969e85845e499affbf686144c6b490b344ab1e`.
Its PE section layout differs. Section-mapped bytes at the five task05 RVAs
`2d00f0`, `162b70`, `0855e0`, `27b1e0`, `269880` differ from the admitted bodies.
Exact 24-byte prefixes of the admitted factory/helper/cache/query entries were
searched in this file; none matched. This only rejects those exact identity
anchors. It does not establish that the variant has no semantically equivalent
producer. Its corresponding function addresses/build binding remain unqualified;
using the original RVAs as function identities would be invalid.

The retained collision tasks0028 through0037 were narrowly searched in code
receipts, span manifests, disassemblies and identity records for these producer
anchors. No retained qualified producer/callback body was found. Current native
movement task03 spans confirm solver callers but do not supply the missing
effective factory. This is a bounded searched-source limit, not a claim about
all possible licensed sources or unrelated inspection output.

## Integration consequence and next boundary

Later bounded [native17 initialization](receiver-polynomial-initialization.md)
qualifies the startup prelude's direct write of reconstructed `edb88320` to
`33dae20`. This resolves the polynomial initializer conditional on normal source
ordering and no intervening mutation. The effective callback at image base,
allocation/vtable/shape and candidate world bindings described above remain open.

Keep alternate receiver access explicitly unresolved in the scene/movement
contract. Do not label static models, terrain, transformed models or dynents
nonparticipating because this binding is missing. Their provider/pose/activation
references require independent scene admission, while already admitted classic
world/brush consumers and bounded terrain/static family investigation can progress.

The highest-value source follow-up is a qualified effective callback/prepopulation
body with provenance, then its receiver shape and candidate/world bindings. No
generic support solver, guessed shape name, process read, original invocation,
asset redistribution or Rust implementation was introduced here.
