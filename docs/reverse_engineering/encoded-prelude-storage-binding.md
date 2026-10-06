# Encoded prelude storage binding

Status: COMPLETE for this bounded static association; fidelity INVESTIGATING.
This establishes an integer CRC-table initialization, not movement execution,
floating-point policy, parameter defaults or a collision receiver producer.

## Source and reproduction

The licensed disk image SHA256 is
`2aff340f22f0c68bf46511c7bbc6167e4bb84524817f54cae08e8f60e6f9afc3`,
image base `400000`. All addresses below are hex. PE sections qualify reads;
zero-fill storage is never treated as file-backed bytes. External receipts under
inspection identity `movement-native-16` contain the bounded scripts, complete
bodies, 155-range identities and independent checks. Original bytes remain external.
See [first record](encoded-prelude-record-qualification.md),
[thread binding](movement-thread-fp-binding.md) and
[receiver producer](movement-collision-receiver-producer.md).

| Complete body RVA | Bytes | SHA256 |
|---|---:|---|
| `1b1630` | 54 | `23ff0a1b4beeeee8503b8fca9946b17ea10caa64da8a230cbfc7337c32cda9c8` |
| `1caae0` | 43 | `d6f4130972e2ccd7925abfac1b73f6fe2ca2d40830b8eceed5882fdb3adc7097` |
| `18fde0` | 6 | `c44c58c54cbaf7097e0d448e2f93a69613f3beea78f34dac7cfb35bd21479de3` |
| `59f5a0` | 232 | `f90941b3f5b84f3b917a314e5c00fa76d3bdc7a274b252aa16d812701e8aff20` |
| `5a8ed0` | 452 | `b7dae0d34a84f75b664e5593b5cdcbc69bc6a359bc1d4c4ab01765191e9b0660` |
| `1d77a0` | 268 | `0b9a1f96400a3d87d7a982235a359f54525f43222a4baeb0d1e14e04d09adf10` |
| `5acda0` | 15 | `edf2f09ce739eb8d4ccc9b93ffbb51c1a978ee85d06e7ed24aa9283264f0c0a5` |
| `015400` | 46 | `5685419ff23bc09fdc2987f37e24d5288a02b58896b846ff933301f5fab65def` |
| `142fa0` | 34 | `2ca7229a6a4624bc467771fb054f80c019a5d9bb628c960789524b26dc4f5f69` |

The second finalizer `5acda0` is 15 bytes, rather than an assumed larger body:
it adds `9b` to its argument and XORs word VA `c8e64c`. The builder ignores
its returned scalar. Shared CRC/complement helpers retain task15 identities.

## Actual arguments and storage

`1b1630` saves the first adapter's returned scalar `01025b40`, calls `18fde0`,
then passes both results to `1caae0`. The numeric first result is used as an
object pointer here; task15 checksum equality alone did not establish this use.

`1caae0` pushes literal one, then calls IAT VA `b6a0fc`, bound by the PE import
to `KERNEL32.dll!GetCurrentThreadId`. That import takes no arguments. The one
remains on the stack and subsequently becomes the third, reflection argument
of `1d77a0`; it is not an OS thread argument. Unsigned division selects the
thread-ID remainder modulo 37. The object at VA `1025b40` is dereferenced to
get pointer-array VA `1025aa8`; the selected array word becomes the destination.
The second adapter's result becomes the polynomial argument. After table
initialization the consumer cleans all three arguments and returns low byte one.

The object and 148-byte array are initialized raw-backed `.data`. Array SHA256:
`98554bde2659aa79fe76223129eabff799fa307ed06915662821269e8ec7582d`.
All 37 entries equal VA `33dae28`. Thus the admitted disk pointer array selects
one shared destination for every possible thread ID; it does not select 37
independent tables. Runtime modification of this initialized array is unqualified.
The destination's 1024-byte interval is `[33dae28,33db228)`, within the section's
virtual zero-fill extent, outside its initialized raw extent. No runtime table
contents were read or asserted.

## Strictly required second record

Builder `59f5a0` uses metadata VA `c8e638`, expected-base header `c8e624`.
Wrapping self-relative addition yields actual base `400000`, equal to expected.
It consumes 155 self-relative ranges from parallel arrays at VAs `c8d750`,
`c8d9d0`, `c8dc50`, `c8ded0` (range pointers, lengths, optional relocation
pointers and lengths). All optional relocation pointers and lengths are zero.
The direct equal-base branch therefore requires no unavailable relocation input.
Ranges total 289280 bytes. The known adapter return VA `58fde5` contributes
little-endian caller offset `18fde5`, giving 289284 bytes total. The shared
CRC update and complement produce scalar `edb88320`. Return flag VA `c8e6bc`
low byte zero selects the checksum itself, without image-base delta addition.
Message SHA256: `d474508369a295a05f8283577b687e27f8ec0da9367a65957b371d7ba41ce5b7`.
This record has no task15 digest gate or indirect payload callback; its short
finalizer's transformed return is unused. No third or fourth record was decoded.

## Table writes and receiver alias check

`1d77a0` receives polynomial `edb88320`, destination `33dae28`, reflection one.
`015400` reverses its 32 bits, producing `04c11db7`. For each index 0..255,
it reflects the eight-bit index, shifts it left 24, performs eight wrapping
left shifts with polynomial XOR when the previous high bit was set, reflects
the final 32-bit value, and stores one DWORD at destination plus index times four.
Its source arithmetic is integer only. Independently reconstructed outputs match
the right-shifting reflected recurrence and task15's static CRC table at VA
`c83cc0`; table SHA256 is
`12f3e0576d447eb37b36d82ba0c1c5481b8f0d12fdc70347ce4a076b229d4c86`.
This comparison establishes table content in the static reconstruction, not an
observation of the destination in an original process.

The receiver dependency VA `33dae20` is a separate DWORD eight bytes before the
shared destination. None of these 256 stores overlaps it. Receiver initializer
`142fa0` reads that polynomial slot, writes state `ffffffff` into its supplied
object, and invokes the same generator with destination object plus four and
reflection one. It does not read the shared destination or derive its polynomial
from the table prelude. This proves a shared generator, not the mutable
polynomial's producer, object allocation, effective factory callback, receiver
vtable or geometry provider. The receiver06 missing callback/base association
therefore remains open.

## Validation and limits

460 independent byte/integer checks passed: all 256 generated entries against a
separate recurrence; full table equality; 155 absent relocation pairs; reconstructed
bytewise checksum versus zlib; all 37 destination aliases; seven synthetic unsigned
thread-ID boundaries; and nonoverlap/return-flag checks. Static PE import binding
and complete bounded source bodies were inspected. No original instructions were
executed or emulated; reconstructed transforms operated only on licensed disk data.

Normal-return table generation is qualified conditional on the source-bound pointer
array and successful prior calls. Consumer/generator bodies install no exception
handler or FP state writer. Faulting or modified pointers are not modeled; task15's
preceding SEH/global finalizer effects remain unresolved. The next two prelude
calls remain outside scope and may have independent effects. Consequently this
association establishes no last-applicable thread FP writer or movement defaults.
A finite next receiver investigation would qualify the producer of DWORD
`33dae20` and its caller/record association, rather than infer it from adjacency.
