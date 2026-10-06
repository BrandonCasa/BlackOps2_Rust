# Movement thread and floating-point binding

Task movement-native-11, 2026-10-05. Fidelity INVESTIGATING. Ordinary movement has
an admitted executing-thread route; its last applicable FP state remains
unresolved. No original process, instruction invocation, play or capture occurred.

## Evidence and reproduction

Use the licensed disk identity and PE mapping from
[movement-native-contract.md](movement-native-contract.md): SHA256
`2aff340f22f0c68bf46511c7bbc6167e4bb84524817f54cae08e8f60e6f9afc3`, image base400000.
All addresses below are RVAs unless labeled VA. Resolve external roots through
ignored configuration. External `movement-native-11/qualification-receipt.json`
records17 complete body hashes, raw offsets and11 independent binding checks.
Raw relative-call/pointer searches only nominate candidates; the admitted chain
below was decoded at complete body boundaries. Prefix/candidate files are not
admitted bodies. No proprietary bytes/disassembly or installation paths are tracked.

Reviewed [FP10](movement-floating-point-policy.md),
[command02](movement-command-update.md), their handoffs, project state and current
concurrent physics diff. FP10 establishes a reachable startup precision request;
that request is not a thread creation contract.

## Executing-thread route

| RVA | Qualified body | Bytes |
| --- | --- | ---: |
| 21fe30 | Registers the frame callback | 29 |
| 218a00 | Creates suspended thread, records callback and resumes | 150 |
| 48fec0 | Shared thread entry wrapper | 76 |
| 2c8050 / 159c20 | Thread context slot initialization / TLS store | 104 / 33 |
| 4926f0 | Frame callback and persistent loop | 984 |
| 2a9e20 / 4301d0 | Frame entity iteration / recursive entity dispatch | 1244 / 682 |
| 06aac0 | Frame entity command adapter | 134 |
| 0241e0 / 0ac2e0 / 095b30 | Command loop / index adapter / entity command consumer | 182 / 117 / 198 |
| 1b1630 | Callback prelude | 54 |
| 5a0770 / 59f5a0 / 59f960 / 59f690 | Encoded record builders reached by prelude | 232 each |

21fe30 supplies callback VA8926f0 to218a00. The creator writes it to VA28503d0,
then invokes imported CreateThread at218a5e with entry VA88fec0, argument10,
creation flags4, default stack size0, null security attributes and thread-ID
output VA28502f0. It stores the handle at VA2850350, performs1a9090's naming
exception helper, checks the handle and resumes through imported ResumeThread
at218a8d. Creation failure returns false; successful creation returns true.
The four preceding imported event-creation calls initialize synchronization
handles. These imported functions are identified by FP10's import table, not
by address resemblance.

48fec0 receives selector10. It invokes the naming helper and assigns current
thread TLS+58 to VA2850110 +20*selector. It calls2c8050(selector), then reads
VA28503a8 +4*selector and calls that callback with selector. For10 this is exactly
VA28503d0, the slot written by218a00. The wrapper returns0 with four-byte argument
cleanup if the callback returns. No CRT begin-thread wrapper is observed in this
creation route; this does not prove that OS/module thread attachment did nothing.

2c8050 installs context slots1/2/3 using per-selector arrays. Slot4 selects one
of two shared addresses by selector==1. 159c20 follows the current module TLS
index VA33e98e4, dereferences TLS+58 and stores the supplied pointer at four times
the supplied slot. These are bookkeeping writes, not FP control changes.

The callback reaches ordinary034ad0 synchronously by two qualified routes:

- At4929cf: 4926f0 ->2a9e20 ->4301d0 ->06aac0 ->034ad0. The entity loop checks
  entity+fb before dispatch. 4301d0 dispatches entities with nonnull+154 after
  its earlier type/attachment branches;06aac0 tests2f1650's result before calling
  movement, setting command time from VA2338f18.
- At4929ff: 4926f0 ->0241e0 ->0ac2e0 ->095b30 ->034ad0. The command loop gates
  participating records;0ac2e0 gates record state5. 095b30 copies15 dwords of
  prior command, invokes1be1d0 for the incoming command, writes command metadata
  and calls034ad0 only when2f1650 returns false. TLS+1a50 is set and cleared.

These edges establish that eligible ordinary updates on these paths execute on
selector10's thread. They do not establish exclusive ownership of every updater
invocation. The raw search also finds034ad0 at21f716 in a separate entity setup
flow; its caller/thread binding is outside this route and remains unresolved.
The additional2a9e20 caller1027c0 is a separate local frame wrapper, not proof of
thread identity. Recursion in4301d0 remains on the same caller thread.

## FP-state boundary and precise missing evidence

No direct fldcw/fnstcw, ldmxcsr/stmxcsr, init or clear-exception opcode occurs in
the admitted registration, creator, entry wrapper, context initializer or TLS
store. This is a bounded instruction claim, not a transitive absence proof.
Their imported naming/event/thread operations and loader callbacks have not been
qualified as FP-state-preserving. In particular the creator's startup-thread
53-bit request from FP10 does not admit worker-thread inheritance, rounding,
exception masks, FTZ or DAZ.

At492716 the frame callback invokes1b1630 before entering its update loop.
That prelude calls four small adapters reaching encoded record builders:

| Prelude adapter | Builder | Record metadata VA | Additional header VA |
| --- | --- | --- | --- |
| 1b89a0 | 5a0770 | ca26c8 | ca26b4 |
| 18fde0 | 59f5a0 | c8e638 | c8e624 |
| 079900 | 59f960 | c92568 | c92554 |
| 1176a0 | 59f690 | c8f5dc | c8f5c8 |

Each complete232-byte builder combines its return-address input, self-relative
record tables and checksum/finalizer consumers. Return values require individual
qualification; they are not all pointers. [Native15](encoded-prelude-record-qualification.md)
reconstructs the first record's scalar01025b40 and verified range association,
without admitting storage semantics or transitive FP effects. The remaining
record associations stay separate. 1b1630 also reaches1caae0 and5ae590. Neither its own54 bytes nor
the immediate TLS initialization selects an FP policy; treating its unresolved
consumers as harmless would be an inference. Subsequent callback helpers before
the first movement call can also have unqualified transitive effects.

The finite next evidence is the current thread entry FP state from loader/module
initializers plus decoded effect/receiver associations for these four prelude
records and relevant pre-movement helpers. A static decoder may qualify those
inputs in a separate task; executing original code or sampling the process is
not authorized. No nearest-rounding or last-writer association is supplied by
this investigation. x87 movement implementation must still require a separately
admitted explicit arithmetic policy rather than a guessed compiler/OS default.

## Independent checks and limits

Independent byte/constant checks passed11 assertions: callback slot and TLS
selector arithmetic, exact supplied callback, imported CreateThread instruction,
suspended flag, selector10 and absence of direct FP control opcodes in five
complete route bodies. Seventeen admitted byte spans decode completely with
recorded identities. These establish bounded source associations; they are not
synthetic movement comparisons or original gameplay evidence. No Rust changed,
so no build/test/Clippy claim is made. Owned-document diff checking passed.
