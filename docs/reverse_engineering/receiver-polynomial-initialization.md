# Receiver polynomial initialization

Status: COMPLETE for the bounded producer association; fidelity INVESTIGATING.
The mutable receiver polynomial has a qualified startup write. Effective receiver
construction and later mutation remain separate evidence requirements.

## Source identity and scope

The admitted licensed disk SHA256 is
`2aff340f22f0c68bf46511c7bbc6167e4bb84524817f54cae08e8f60e6f9afc3`,
PE image base `400000`. Addresses below are hex. External inspection identity
`movement-native-17` retains bounded scripts, raw spans and per-range receipts;
no proprietary bytes or local paths are tracked. Read current architecture and
[FP11](movement-thread-fp-binding.md),
[native15](encoded-prelude-record-qualification.md),
[native16](encoded-prelude-storage-binding.md) and
[receiver06](movement-collision-receiver-producer.md) before reconstruction.

| Complete body RVA | Bytes | SHA256 |
|---|---:|---|
| `079900` | 6 | `e7642834ee1a8b69550b55cbc4ec72d5e9977d1e2a21366d40b12f0a3c2a4809` |
| `59f960` | 232 | `a00a33d5d65e41ab61047dc9f9473296a876a652deb4b6f75a6db7d976e02c91` |
| `5a9610` | 452 | `bd28ef9be65c2646c08c1a314a4a92a95195b339b9c7141c384cfad917eec8d0` |
| `5acde0` | 15 | `2587766857f3a22e538df6bf40b0e833b3e65a1e01974b58f7e813eeb7f9da19` |
| `5ae590` | 20 | `b46125271499020a329af0ae882c8d5924a9b579a7391e8a7896e9b4234186ab` |
| `5ae4a0` | 112 | `315c17496e8e04b6fdc7c4dbc7ebb87d4e36047ff272139726cb2aeda10c1e0a` |
| `5ae510` | 111 | `9dd708ab566a158f85d4dbb4c5c911007e65a71fa27e7a99c0ad370eed7f3b91` |
| `5ae030` | 85 | `3deadef150d2256d08bab472bdcbb518355a15035923bd249900368e122dfd45` |
| `5adfd0` | 85 | `083dc3f6b2b9f5c63a4c84a425c92e7028a1761b92e5ec25dd7ea3112f3fee97` |

`5ae510` includes the deliberate interior breakpoint and its subsequent cleanup
through its return; stopping disassembly at that breakpoint would omit the
exceptional continuation. `5ae4a0` ends at its own return; scanning until later
padding would incorrectly include the next function. Prior task identities bind
`1b1630`, CRC initializer/update/complement and receiver initializer `142fa0`.

## Exact caller and write

FP11 binds selector10's thread wrapper to callback `4926f0`. At `492716`, that
callback calls prelude `1b1630` before its update loop. This is a source-bound
worker startup association, not proof of a running thread's sampled state.

After the first two record adapters and table consumer, `1b1630` calls `079900`
at `1b1644`. It stores the returned EAX directly into DWORD VA `33dae20` at
`1b1649`. The six-byte adapter calls builder `59f960`; its return VA is `479905`.
There is no transformation between builder return and store. Thus normal source
flow supplies the receiver polynomial before the following adapter/consumer.
The write is a runtime initialization from a statically reconstructible record,
not an inferred native registration default or a file-backed initial DWORD.
The slot is in the PE section's virtual zero-fill extent; its active value is
not read from disk. This task does not establish all writers or persistence.

## Individually bound third record

The metadata-relative word at VA `c92568` yields base `400000`; header VA
`c92554` contains the same expected base. Builder `59f960` consumes 159 ranges
(offsets zero through `278`, stride four), using arrays at VAs `c91620`,
`c918b0`, `c91b40`, `c91dd0` for self-relative pointers, lengths, optional
relocation pointers and lengths. All 159 optional relocation pairs are zero.
Bounded raw-backed range reads total 265728 bytes. The caller offset `079905`
is appended as four little-endian bytes, giving 265732 bytes total.

The source CRC object starts `ffffffff`, follows the independently qualified
byte-update/complement boundary, and yields `edb88320`. Message SHA256:
`099017989dfcd181f7b3642f2cbaf108cec862d8776da6c4cf05cc2fca2ab7a9`.
Return flag VA `c925ec` low byte zero returns that scalar without base-delta
addition. Finalizer `5acde0` adds `9f` and XORs immediate address value `c9257c`;
its returned scalar is ignored by the builder. It does not dereference that
immediate or perform a digest gate. No runtime seed or indirect callback is
needed on this admitted equal-base record path. Its checksum equality with the
second record is corroboration, not the producer association: the actual store
and this record's own caller establish the binding independently.

## Receiver consequence and bounded later effects

Receiver initializer `142fa0` reads DWORD `33dae20` and generates a reflected
lookup table at its supplied object's address plus four. After the qualified
prelude store, that read's polynomial input is `edb88320`, conditional on normal
ordering and no intervening mutation. Native16's separate shared-table writes
at `[33dae28,33db228)` do not establish this slot; native17's direct store does.

This closes the previously missing static initializer for the checksum
polynomial used by `162b70 -> 142fa0`. It does not close the separately missing
effective receiver callback: receiver06 still derives callback address `400000`,
which is the admitted disk's PE header, rather than a qualified constructor.
Metadata transformation, exception-mediated effects and resulting receiver
allocation/vtable/candidate geometry remain unresolved.

The later fourth record and `5ae590` are unnecessary to derive the earlier
store and were not decoded. Bounded inspection of the latter establishes a
call to `5ae4a0` and a conditional call to `5ae510` if return equals one. Both
helpers install SEH frames and obtain cached objects from `5ae030`/`5adfd0`
(storage VAs `33dbd80`/`33dbd84`), then call virtual slots zero, four and eight.
The new-object path installs vtable VA `ca4bac`; `5ae510` reaches an interior
breakpoint before SEH cleanup. The effects of these object methods, helper
`5ae370`, and exception callbacks `5ae170`/`5ae270` are not qualified here.
None of these inspected direct bodies writes `33dae20`, but their indirect and
exceptional effects prevent a claim of immutable polynomial or final FP state.
This is a finite next source boundary, not proof that these effects are absent
or impossible to reconstruct. No fourth-record value is needed by the direct
polynomial store and none was inferred.

## Validation and limits

320 independent byte/integer checks passed: base association, all 159 bounded
range lengths, all 159 absent relocation pairs, and reconstructed bytewise
checksum versus independent zlib. Complete record/helper bodies and exact caller
store were inspected; native16 supplies independently checked CRC-table content.
No original instructions were executed or emulated. The reconstructed checksum
operated only on licensed disk bytes; source control-flow descriptions are
static evidence, not observed execution or game comparison.

There is no new Rust kernel, executable movement profile, last-applicable FP
policy or usable collision receiver assertion. A follow-up should qualify
`5ae590`'s static vtable/SEH consumers if persistence or pre-loop effects are
needed, or the effective callback/prepopulation producer if receiver creation
is the objective. Do not treat this initialization as resolving either question.
