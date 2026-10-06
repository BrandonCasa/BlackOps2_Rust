# One encoded prelude record: static checksum and range association

Task movement-native-15, 2026-10-05. Fidelity INVESTIGATING. For the admitted disk
and known1b89a0 caller, builder5a0770's checksum, verification range and scalar
return reconstruct from static inputs. No original instruction stream was
executed or emulated; no callback/payload was invoked. Reconstructed byte/integer
transforms read licensed files only, with proprietary outputs retained externally.
This is one record, not a whole-program decoder or gameplay/FP behavior proof.

## Identity, scope and reproducibility

Licensed disk SHA256
`2aff340f22f0c68bf46511c7bbc6167e4bb84524817f54cae08e8f60e6f9afc3`, image base400000.
Use PE section mappings and ignored source/evidence locators. Reviewed project
state/current concurrent diff, [FP11](movement-thread-fp-binding.md) and
[receiver06](movement-collision-receiver-producer.md). External
`movement-native-15/qualification-receipt.json` records19 complete body identities
and111 independent/structural checks. `ranges.private.json`/`decoded.private.json`
retain range details, and `payload.private.bin` retains the referenced bytes.
The bounded decoder reads179 entries, each <=8192 bytes, 283904 aggregate bytes,
plus four caller-offset bytes and one1792-byte verification range. No other three
prelude records were read or reconstructed.

| RVA | Source role | Bytes |
| --- | --- | ---: |
| 5a0770 | One prelude builder | 232 |
| 5ac8d0 / 027db0 | Checksum init / complement | 9 / 7 |
| 5ab140 | Range feeding and relocation normalization | 452 |
| 5ac900 | Checksum update code | 538 |
| 5a3bd0 / 5a3c30 | Bounded range copy / overlap advance | 83 / 130 |
| 228be0 | Bit reversal key transform | 97 |
| 59e9c0 | Verification finalizer and compare branching | 197 |
| 03f240 / 5a0a40 | SEH-wrapped explicit callback / range digest feeding | 95 / 452 |
| 079560 / 192f90 | Digest init/update adapters | 16 / 22 |
| 00e960 / 00a8b0 / 5b0a80 | Digest init / buffered update / compression | 40 / 191 / 1654 |
| 014fe0 / 042680 / 0f5bd0 | Digest final wrapper / finalization / comparison | 14 / 150 / 161 |

Checksum code ends at5acb1a; two alignment bytes precede an eight-entry jump table
at VA9acb1c,32 bytes. That table is data, not adjacent function code. The early
exploratory padding span crossed it and included another function; it is excluded.
Digest init079560 similarly ends after16 bytes, before adjacent079570.

## Abstract record schema and scalar return

This builder initializes a four-byte checksum state to ffffffff. Metadata is a
set of parallel32-bit arrays,179 entries each, indexed by byte offsets0..2c8:

| Static VA | Meaning |
| --- | --- |
| ca15a0 | Self-relative range word: address of word + stored word modulo2^32 |
| ca1880 | Referenced byte length |
| ca1b60 | Optional self-relative relocation stream; raw0 means null |
| ca1e40 | Relocation-stream length |
| ca26c8 | Self-relative actual image-base word |
| ca26b4 | Expected image base |
| ca26fc / ca2710 | Final keyed relative range word / byte length |
| ca2724 / ca2738 | Optional keyed relocation stream / byte length |
| ca26dc | Expected16-byte verification digest |
| ca274c | Low-byte return-offset flag |

All pointer arithmetic here is explicit wrapping32-bit arithmetic. The actual
base is `ca26c8 + word(ca26c8)` modulo2^32. It resolves400000 in this disk, equal
to expected base400000. Let delta=actual-expected and callerOffset=return-actual.
For the admitted direct caller1b89a0, the return VA is5b89a5, so callerOffset is
1b89a5. Return-address provenance comes from that complete six-byte call/return
adapter, not from sampling a stack.

For each range5ab140 receives context, actual base, resolved range, byte count,
optional relocation stream/count, expected base and flag0. Since actual==expected,
it directly feeds that complete byte range to5ac900, in metadata order. No sorting,
deduplication or skipped overlapping range is admitted. All179 relocation words
and lengths are zero in this record, but the direct branch already makes their
normalization irrelevant to this admitted reconstruction.

If bases differ, source copies bounded chunks with four-byte overlap and traverses
PE-style relocation blocks (page RVA, block size, packed16-bit entries). Type3
entries subtract delta from the copied32-bit word before feeding. Initial range
copy starts at range-4 with prefix skip4; effective chunk ceiling is f7f with a
four-byte overlap when another chunk is required. This branch is source described
only, not a claim that this task decoded a differently
loaded image. Dynamic image/pointer mutation or relocation-normalized output needs
its own qualified inputs.

5ac900 uses table VA c83cc0: update state is
`(state >> 8) XOR table[(state XOR byte) & ff]`.
Its bulk path XORs a little-endian word then performs four table/shift updates,
and handles length/alignment residuals through its internal jump table. All256
static table entries match reflected polynomial edb88320; the table's section is
readonly.rdata. Its1024-byte SHA256 is
`12f3e0576d447eb37b36d82ba0c1c5481b8f0d12fdc70347ce4a076b229d4c86`.
This fixed table is distinct from receiver06's mutable polynomial-derived table.

After283904 range bytes, append callerOffset as four little-endian bytes; then
027db0 complements state. The result is **01025b40**. The bit-reversal adapter
228be0 swaps1/2/4/8-bit groups then rotates16; finalizer XORs07000700, yielding
key **05da4780**. Builder retains the checksum scalar across finalization.
Return flag ca274c's low byte is nonzero, so final scalar is checksum+delta modulo
2^32; delta0 gives01025b40. That number coincides with VA1025b40 used as an address
by the following prelude consumer. Its object/storage role is not inferred here.
The builder's returned value is not the address of the verified range.

## Verification range and direct callback association

Finalizer decodes the final range as
`ca26fc + (word(ca26fc) XOR key)` modulo2^32. For this input it resolves VA9a3800,
length700 (1792 bytes), SHA256
`4ca7789808d56bf920496237c754a1266bbb5f552d6d56e30fca968c62c13d96`.
The optional keyed relocation word/length are zero. This is a **referenced range
being hashed**, not a decrypted or executed instruction payload.

59e9c0 initializes a68-byte digest context and first feeds the four little-endian
key bytes. It supplies an explicit direct callback VA9a0a40 to03f240, along with
context, actual base, decoded range/count, optional relocation data, expected
base and flag0.03f240 constructs an SEH frame from VA c32820/c08dd4, forwards
those eight arguments and calls that supplied callback. The normal callback here
is statically identified5a0a40; it is not receiver06's unidentified base-address
callback. Bases equal takes the direct digest-feed branch, without chunk copying
or relocation adjustments. The callback's result is not used as a function target.

The digest initialization words, four Boolean round families, all64 constants
and64 rotation counts,64-byte block update, little-endian bit-length padding and
16-byte output qualify MD5. Independent standard digest calculation of
`LE32(key) || referenced range` exactly equals the complete expected16-byte record
at ca26dc. Raw digest/range bytes remain external. Equality makes0f5bd0 return0;
normal finalizer flow then calls044a30 with arguments3/b3. Inequality reaches
0c0160 followed by1297e0 with two zeros. These names are addresses, not an inferred
success/failure/gameplay meaning for those global-state consumers.

## Validation and limits

Independent checks passed: all256 table entries, decoded bases/known caller,
179 bounded ranges, no relocation entries, byte-stream CRC against zlib,65
synthetic word/byte update cases,35 bit-reversal cases, keyed range association,
complete digest equality and64 compression constants/rotations. Total111 named
checks and19 admitted body identities are recorded. The checksum message
(283908 bytes) SHA256 is
`0c4ec9bd5ba2bd0ea71002ae03668ddc17c71f2af998b445a4ab553b4bccce9b`.
Owned document whitespace/path/diff checks passed; no Rust/build gates or
original comparison claimed.

For this disk/known caller, no unknown runtime seed is needed to reconstruct the
normal checksum/key/verified-range/scalar-return association. This narrows FP11's
first prelude record gap. It does **not** qualify runtime memory mutations,
loader callbacks, SEH exceptional effects, the later044a30/0c0160/1297e0 global
state, storage at scalar-valued VA1025b40, or another prelude's returned value.
The next finite source work is the following consumer1caae0's association of this
resolved storage with its second argument, separately qualifying that other
argument/record. No worker FP or numerical default is inferred from a matching
verification digest, and receiver06's distinct unresolved callback remains open.
