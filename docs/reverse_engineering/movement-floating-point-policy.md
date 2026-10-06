# Movement floating-point policy qualification

Task movement-native-10, 2026-10-05. Fidelity INVESTIGATING. This separates
reachable startup control changes from the unresolved policy at movement entry.
No original process read, invocation, play/capture or original instruction
execution occurred. Independent reconstruction uses integer bit operations only.

## Identity and reproduction

Use [movement-native-contract.md](movement-native-contract.md) for the licensed
disk identity: SHA256
`2aff340f22f0c68bf46511c7bbc6167e4bb84524817f54cae08e8f60e6f9afc3`.
The image base is400000. All RVAs below refer to this one disk image, not the
Steam image or a sampled runtime object. PE section mapping determines raw file
offsets; external `movement-native-10/qualification-receipt.json` records those
offsets,13 bounded body lengths/hashes and independent checks. Resolve external
root through ignored configuration. No binary/disassembly/import dump is tracked.

Relevant prior contracts are
[movement-grounded-contract.md](movement-grounded-contract.md),
[movement-baseline-parameters.md](movement-baseline-parameters.md) and
[movement-stance-contract.md](movement-stance-contract.md). Their x87 precision,
rounding and thread-policy gaps are narrowed below, not erased.

## Reachable startup precision change

| RVA | Observed responsibility | Admitted bytes |
| --- | --- | ---: |
| 678fbc | PE entry | 10 |
| 678e4f | Startup control flow before application entry | 365 |
| 6747ed | Initializer dispatch with FP hook | 151 |
| 66cf2c / 66cecc | FP hook / math callback-table initializer | 25 / 96 |
| 67c789 | Precision change request | 40 |
| 690fe8 | Checked abstract-control adapter | 95 |
| 694018 / 693eea | Read/merge/write control / x87 translation | 786 / 142 |
| 688d40 / 688cb0 / 688cf0 | Hook-image guard / header / section check | 187 / 53 / 68 |
| 66e3a0 | Movement-referenced float-to-integer helper, including fallback | 171 |

Complete function boundaries exclude adjacent entry points and padding. The
66e3a0 span includes its shared fallback and alternate entry66e3bc; it ends after
66e44a, before an unrelated adjacent function.

PE entry678fbc calls68f2f3 (cookie initialization lead) then jumps678e4f. Startup
calls6747ed with argument1 before application entry1965f0.6747ed checks pointer
slot VA ccfe50, verifies the slot belongs to nonwritable image memory via688d40,
and calls its initialized function value VA a6cf2c (RVA66cf2c). Slot is in PE
.rdata, RVA8cfe50, section characteristics40000040, with write flag80000000 absent.
The guard checks this same image's MZ/PE/PE32 headers and containing section.
These bindings give a reachable source hook for the ordinary successful startup
path; they are not a runtime sample of an assumed global pointer.

66cf2c initializes math callback pointers through66cecc. With its passed argument1,
it calls67c789, then clears x87 pending exception flags via fnclex.67c789 calls
690fe8 with output pointer0, requested abstract value10000 and mask30000. That
adapter validates the value/mask, forwards them to694018 and returns success.
The mask changes only the abstract x87 precision field.694018 reads fnstcw,
translates defined fields, merges supplied fields and, if changed, calls693eea
and fldcw, then reads the result again.

Source translation maps abstract10000 to hardware x87 precision bits0200,
corresponding to **53 significand bits**. The mask does not select rounding,
exception masks or infinity control; their defined incoming values are retained.
Reserved bits are not claimed to be bit-for-bit retained by the translation.
This is a demonstrated startup-thread precision selection, not a universal
movement-thread control-word value such as an assumed027f/037f.

694018 separately masks requested SSE fields with0308031f. Precision mask30000
has no overlap, so this request does not select any MXCSR field to change.
The helper may read stmxcsr, but the unchanged SSE merge bypasses its MXCSR writer.
Thus this hook does **not** establish SSE rounding, flush-to-zero, denormals-are-zero
or SSE exception masks. fnclex clears x87 status, not its exception-mask policy.

## What remains unknown at movement entry

The startup precision request preserves inherited x87 rounding. No qualified
call before this hook establishes nearest rounding. PE TLS callback array at
VA b6ab88 begins with a zero pointer; it supplies no callback in this image.
Imported modules can still execute their own initialization, and this source
inspection establishes no operating-system loader FP policy. No default MSVC,
Windows, CPU-reset or Rust-thread control setting is admitted by familiarity.

After the hook,6747ed iterates additional initializer ranges, including the
315 nonzero entries in VA b6a66c..b6ab5c, before returning. Application entry1965f0
performs other indirect/library initialization before its loop. Their complete
transitive control-word effects are not qualified by this bounded task. The
presence of a startup hook does not prove an unchanged policy after all callbacks.

The PE imports CreateThread at IAT VA b6a204. Eleven byte-pattern candidate call
sites were retained; none is treated as a qualified movement-thread assignment.
Whether ordinary command owner034ad0 executes on the startup thread or another
thread, which creation wrapper establishes its state, and whether intervening
callbacks alter that state remain exact missing associations. Startup-thread
precision must not be projected onto all worker threads. Thread inheritance is
not inferred from an OS convention or from a absence-of-writes scan.

A raw opcode search produced1012 FP-looking candidates, many inside other
instructions/data. A linear instruction-boundary scan with skip-data produced471
candidates, still not a reachable call graph. It found no pre-CRT-range ldmxcsr or
fninit at those linear boundaries; that is a scan result, **not proof of absence**
or a nearest/gradual-underflow policy. Admission uses bounded reachable bodies,
not either candidate list. A representative engine span021631..02167b temporarily
ORs x87 rounding bits c00 for integer conversion and restores a saved word; that
local conversion is not evidence of a global rounding change. Other unqualified
candidate wrappers remain external leads.

The only direct690fe8 call candidate is the admitted67c799 precision request;
694018 call candidates occur in690fe8. Other writable-control adapters6846bc and
68473f appear in math/runtime consumers. Their mere availability does not bind
a movement policy, and indirect/encoded callbacks are not excluded by that scan.

## Instruction-level consequences for existing kernels

Native friction, projection and speed consumers retain x87 arithmetic and explicit
f32 stores documented in native03/04. Startup53-bit precision narrows the plausible
significand setting on one source path, but x87's extended exponent range and
store rounding still differ from blindly evaluating all operations as Rust f32.
Choosing ordinary Rust f64 is also not yet a proof of original evaluation across
rounding/underflow/exception domains. No numerical motor should silently repair
this uncertainty with a generic controller or a compiler default.

Scalar SSE mulss/addss/divss/sqrtss/cvtsi2ss/cvtdq2ps retain their instruction and
operand/store order. Those boundaries do not themselves establish incoming MXCSR.
Finite inputs alone do not prove denormal-free intermediates or nearest rounding.
Previously implemented timing/acceleration and synthetic bit fixtures remain
bounded independent native-flow reconstructions, not original comparison evidence.
Explicitly supplied/checked FP domains are a possible later safe API, not an
implicit newly approved whole-game policy.

Movement-referenced conversion66e3a0 tests capability global VA43769c8. Nonzero
uses an x87 store to f64 followed by cvttsd2si (explicit integer truncation);
zero selects an x87 fistp/shared correction fallback. The capability global's
startup association and active value, fallback exceptional-input behavior and
store-rounding policy remain unqualified. Do not treat every movement integer
conversion as an unconditioned Rust cast. The separate alternate entry66e3bc
also checks x87 mask bits before selecting the fast path.

## Validation and precise next evidence

Independent integer reconstruction passed 1,536 defined x87-control combinations:
three legal precision choices, four incoming rounding choices,64 exception-mask
combinations and two infinity choices. The admitted request selects0200 while
preserving defined rounding/mask/infinity fields. Additional table/parameter
checks validated hook slot, nonwritable section, null TLS list, accepted request
and zero SSE-field overlap. No native instructions executed in these checks.
Owned-document `git diff --check` passed; no Rust/build changes were made.

The remaining policy task is finite: associate034ad0's ordinary command dispatch
with its executing thread/creation wrapper and retained startup callbacks, then
qualify the last applicable control-word/MXCSR writer before that dispatch. For
x87 kernels, admit rounding and exception/underflow domain as well as precision;
for SSE, admit MXCSR rounding/FTZ/DAZ/capability. Missing callbacks/encoded storage
must yield a precise source gap, not assumed inherited values. Dvar defaults,
new motor kernels, live observation and original comparison are outside this task.

