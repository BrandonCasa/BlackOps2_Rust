# Original active-view consumer investigation

Recorded: 2026-10-05. Task0025 is **PARTIAL**: the bounded offline route did
not establish an original camera/projection consumer, locator or scalar layout.
It therefore does not enable the asymmetric Origins patch calibration in
[findings0009](0009-origins-coordinate-evidence.md). Data/access understanding
remains **INVESTIGATING**; overall, gameplay and visual fidelity remain **UNKNOWN**.
The full faithful Origins objective in stable safe Rust with Bevy remains active.

## Question and admitted source

Following the fallback in [findings0010](0010-original-coordinate-calibration-access.md),
the question was whether one original registration/reference/producer/consumer
chain could identify an active view and its camera/projection record. Static
declarations, exporter addresses and familiar names are insufficient.

Only the previously catalogued, configured-root-relative `t6zm.exe` was admitted:
12,543,488 bytes, SHA-256
`f6f7104af2bd0c2b931ea1e739969e85845e499affbf686144c6b490b344ab1e`.
The initial admission and static session read the file through read-only,
unbuffered handles. Size/hash and size/write-time/device/file-ID stability were
checked before interpreting the session's retained bytes. The same session handle
was rehashed before its final metadata check; its retained receipt matches admission.
The fourth preservation hash/assertion preceded a later documentation-regex failure.
Its successful assertion passage is control-flow evidence from the retained helper
and exception type; no separate numeric fourth-pass receipt was saved. No fifth
original pass is taken. These are four full identity passes with 65,536-byte
blocks and a 16-MiB limit per pass. No other original executable, container,
snapshot, script or runtime state was read.

The earlier catalogue's Steam Zombies app212910/build24784239 remains historical
context; this task binds executable bytes and does not refresh the manifest.
The ignored configuration is parsed as UTF-8 TOML, and the fresh external output
area is required to resolve outside the repository. Original bytes, instructions,
full strings, preferred absolute addresses and private paths remain external.
Tracked RVAs below are only noncopyrightable location measurements.

## Tool and procedure

The existing Task0024 build closure supplied the selected `dumpbin.exe`; actual
bytes/hash match its retained receipt. It is 23,376 bytes, SHA-256
`8d463eb6d353f3fcba65fb1894338a765ff4530c39a6d917d48b1b5f03a83aae`.
Bounded help reports `/DISASM` and `/RANGE:vaMin[,vaMax]`; its exit1100 and
950 combined output bytes are retained. No disassembler or SDK was installed.
The bundled dependency runtime supplies Python, 107,312 bytes, SHA-256
`5c14dc10a9b0076af8ff94f049340fb706029e92b5afda89ccc735ea9b5d26a7`.
This interpreter differs from Task0024's interpreter; their identities are not
equated. The first evidence-freeze helper incorrectly required that equality,
failed before sealing, and is retained with its failure/discriminator. The
corrected helper records the actual bundled identity.

The task-specific external analyzer checks DOS/PE/COFF/PE32 fields, bounded
section tables, raw and virtual spans/overlaps and relocation blocks/types.
RVA-to-file translation admits exactly one file-backed span. It never allocates
or traverses the large virtual-only data tail. The parser permits at most32
sections, a 128-MiB image span and500,000 HIGHLOW entries; unsupported relocation
types and unbacked targets reject. These are logical bounds, not hard OS memory
or process supervision guarantees.

The interactive query path is a task helper, not an admitted general inspector:
the actual requests are trusted, fixed positive spans. Its file-offset helper
does not independently reject a negative requested size. The disassembler's
1-MiB output check occurs after `capture_output` returns and supplies no streaming
or hard concurrent allocation cap. Synthetic parser tests do not establish a
complete malformed-query or native-tool containment contract.

Its string search is a finite exact byte search inside the eight file-backed
section spans, totaling12,531,720 bytes. The sole selected anchor is the
NUL-terminated `cg_fov` candidate explicitly admitted as a hypothesis. It occurs
once, at RVA`0x717d94` in `.rdata`. This actual occurrence supports selecting the
candidate for a reference test; it establishes neither registration nor camera
ownership.

The reference search tests every possible in-section start for the four-byte,
little-endian preferred virtual address of that occurrence, without alignment
filtering. It finds **zero** matches across those same eight spans. It excludes
headers, gaps, cross-section encodings, virtual/BSS bytes, relative addressing,
computed/encoded references and runtime-generated code. No second or third
anchor is selected, and no reference/consumer chain is recovered. Zero matches
are a negative result for this exact encoding and coverage, not proof that the
original has no FOV setting or camera consumer.

## Actual static viability observations

The file's relocation directory is empty and PE `DYNAMIC_BASE` is clear. This
describes its static image; it does not validate runtime loading at a particular
address or supply an active-view locator. The `.text` file-backed span begins at
RVA`0x1000`; `.interpr` begins at RVA`0x700000`. Their measured raw sizes are
7,332,864 and26,624 bytes. Section names are labels, not semantic proof.

Three original range requests, each512 bytes, were retained exactly. The tool is
invoked with `/nologo /disasm:nobytes` and an inclusive preferred-address range
derived from the admitted PE base plus the requested RVA. All three exit0.
The tool reopens the admitted pathname; it does not consume the analyzer's held
handle. No retained reopened-name/file-ID check around each tool open or
share-denial observation binds the tool's actual input. Final same-handle and
preservation assertion cannot exclude a
transient pathname substitution or turn that separate open into a handle-bound
snapshot. The static result is conditional on that admitted pathname continuing
to identify the original during the tool invocation.

| Requested RVA interval, half-open | Decoded rows | Privileged/interrupt rows | Direct call/jump targets outside image |
| --- | ---: | ---: | ---: |
| `0x1000..0x1200` |196|19|3 of3|
| `0x700000..0x700200` |167|0|0 of13|
| `0x4f2240..0x4f2440` |242|20|1 of2|

Every decoded row starts inside its requested interval. The last row starts at
RVA`0x11ff`, `0x7001ff` and `0x4f243f`, respectively, one byte before each end.
The retained `nobytes` output does not record instruction lengths, so final
instructions may cross those ends; their crossing widths are unmeasured.
1,536 is the total requested range width, not an exact consumed/disassembled
byte guarantee. The tool's internal read/decoding volume is not instrumented.

The adjacent executable-section sample has coherent in-image transfers,
including a call at RVA`0x7000ca` to RVA`0x4f2240`. Only that directly reached
callee is followed. Its bounded decode, like the `.text`-start sample, contains
many privileged/interrupt operations and implausible transfers. No reliable
function/control-flow interpretation or field-access semantics is established.
The metrics are heuristic decode-quality observations: successful x86 decoding
is possible on arbitrary data, and even an exact call target can reach bytes
whose disk representation is not usable code. No camera ownership is assigned
to this adjacent-section chain.

Protection, encoding, non-code spans or another disk/runtime representation
could explain these observations. **Protection remains a hypothesis.** The
task does not prove a packer/cipher, the extent of affected code, or that every
static route is unavailable. The permitted candidate fails to reach registration;
the callee viability check supplies no trustworthy replacement. Investigation
stops here, without unpacking, emulation, execution or broader code discovery.

Actual scope is one anchor, zero reference candidates, three disassembly windows
totaling1,536 requested code bytes and one directly followed callee. None of the
three-anchor/128-reference/32-function/1-MiB bounds is exhausted. The full identity
passes are separate from these bounded interpretation reads.

## Missing measurement contract and reassessment

No camera origin, ordered basis/matrix, angle field, projection scalar, viewport,
record width/order, producer, frame publication rule or active-view selection is
established. There is consequently no valid future memory-read route from this
task: record spans, pointer/relocation treatment, partial-read rejection, loaded
Origins association and cleanup remain undefined. No guessed offset is proposed.
Runtime view registration, gravity/up, angle semantics, one-sided front/cull and
independent physical units remain separate unresolved calibration fields.

The explicit reassessment is that **this offline fallback did not enable camera
calibration, and no further capture prerequisite is selected now**. Outer review
selects the already listed Origins clipmap's single-brush plane/bounds boundary
as the next independent original-data investigation toward standing movement
and collision. Establish the primitive's observed structure and geometric
consistency before a separate Rust/Bevy adapter or collision implementation.
Original player masks, capsule dimensions, speed, timing, units and live
collision behavior remain unmeasured; the data investigation cannot supply them
by assumption.

The visible rendering milestone remains one representative asymmetric Origins
patch with evidence-backed orientation, scale and camera/visible-side
correspondence in the existing Bevy diagnostic tooling. Revisit its access only
through a separately bounded route with new discriminating evidence. The prior
SDK preparation remains accepted, with its runtime/profile/thread-supervision/
original-compatibility/loaded-map limits intact; it is not extended by this task.
Bevy retains App/ECS/schedules/assets/rendering ownership; seven safe Rust crates,
the raw snapshot/Mesh contract and diagnostic camera policies remain unchanged.

## Reproduction and validation

External `0025-original-active-view-consumer` retains the actual bootstrap,
private configured source/tool selection, predeclared policy, identity receipts,
PE/relocation metadata, negative search, exact tool output, parsed instruction
metadata, range metrics, failures and synthetic tests. `evidence-freeze.json`
seals35 artifacts; its SHA-256 is
`aebf74179da8d708759564cf20788adc85f4c88cae4592917cf5dc526f5567c5`.
Subsequent documentation/preservation records have a separate terminal manifest.
Final candidate and terminal identities are supplied in the handoff delivery.

The first final-validation helper fails on an invalid mid-alternation regex flag
after its original identity assertion. Its exact source/failure is retained.
The corrected documentation-only validation uses the retained same-handle terminal
identity and reruns no original hash, search or disassembler operation.

The29 synthetic parser cases pass, including valid file-backed/BSS behavior,
truncated/unsupported headers, directory/section/image caps, negative and overflow
spans, raw/header and raw/virtual overlap, HIGHLOW/ABSOLUTE relocations, unsupported
types, malformed block sizes/alignment and unbacked relocation targets. The
standalone test helper AST-selects only the four pure parser functions and reruns
those29 cases without reading configuration/originals or invoking a native tool.
These tests establish bounded parser behavior; they do not validate original
camera semantics or protection hypotheses.

Preservation covers112 starting tracked/config records, including109 non-owned
records, and all129 prior Task0024 worker artifacts plus its terminal manifest and
78 parent-review artifacts plus its terminal manifest. Bytes/hash/mtime match.
Prior terminal hashes remain
`5fbe4fcc8ac359a9ebd9011df97e502ca7aabaaa202baba27563c95ad10fa3c8`
and `300b53d8769c7c876dce6d607eabed75c2fffa0dc2b8be5ac0b93f3ccec12748`.
Exact five-document scope, LF, relative links, ten handoff headings, privacy/root
variants and whitespace are checked. Only own schemas, identifiers, measurements
and reasoning enter the candidates; original bodies and private dictionary keys
and values are excluded. Unchanged Cargo74-test/GPU/inspector gates are explicitly
SKIPPED. Original runtime/capture/calibration/gameplay/visual comparison and
performance checks are SKIPPED; no playable or calibrated result is claimed.

Outer review accepts this scoped PARTIAL result. All43 terminal artifacts and
five sealed candidates match independently;109 non-owned records and the prior
Task0024 worker129/root78 freezes remain unchanged. Copied AST-only tests pass
all29 cases under native/process launch traps with zero attempts. Retained
negative-search/range metrics and actual tool identity match independently,
without another original scan. A synthetic regex check reproduces the preserved
validator failure; source ordering supports only the stated fourth-pass
control-flow inference. All114 relative links, LF/privacy, ten headings, exact
scope and whitespace pass. Review records stay external in
`0025-original-active-view-parent-review`; no fidelity status is promoted.
