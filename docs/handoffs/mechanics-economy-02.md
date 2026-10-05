TASK:
Investigate one modifier-free Origins Pack-a-Punch payment-to-weapon-return
transition and generator-loss shutdown deferral.

STATUS:
COMPLETE

FINDINGS:
- Origins explicitly registers/includes MP40 and upgraded MP40; shared code
  establishes normal first-upgrade price 5000, payment before temporary removal,
  buyer retrieval, inventory grant and default ready timeout 15 script seconds.
- Origins enables at exactly six player-controlled zones; on loss it changes HUD,
  waits for in-use clear and then switches trigger off, without directly sending
  the generic destructive power-off event in that body.
- Taken notification precedes inventory grant: in-use clear does not prove grant
  completed. Generic power-off is a separate callback path with unresolved Origins
  dispatch; timeout/disconnect cleanup is not a proven global refund policy.

EVIDENCE:
- Read current Git status/diff, architecture/proposal, previous economy handoff,
  and relevant retained patch/tomb compiled script views.
- Source/build/hashes, function/opcode anchors, staged rules and native limits in
  [Pack-a-Punch record](../reverse_engineering/origins-mechanics/economy/pack-a-punch-02.md).
- Newly decoded weapons/power and disassembled weapons/tomb in unique external
  mechanics-economy-02; reused perk/capture/score/magicbox/utility material.
- Rehashed eight compiled dependencies, both containers and both tool binaries;
  external identity receipt includes the four newly generated views.
- No live game/process/input, timing measurement or original comparison.

CHANGES:
- Added pack-a-punch-02.md, synthetic-pap-cases.json, check_synthetic_pap.py and
  this handoff. Added a coordinator-requested pointer in the prior perk record to
  generic power-off pause evidence and unresolved Origins dispatcher behavior.
  No production API, shared index or architecture edit.
- User-created model keeps payment, removal, readiness, taken, cleanup and grant
  separate; it does not copy original code or assert native serialization.

VALIDATION:
- Four bounded GSC Tool invocations succeeded; critical price, flag/debit order,
  removal, notification-before-grant and deferred trigger shutdown checked against
  disassembly. Same-tool disassembly is corroboration, not independent decoding.
- `python docs/reverse_engineering/origins-mechanics/economy/check_synthetic_pap.py`
  exits 0: PASS, case_count 15. Boundary/replay/conservation and staged shutdown
  checks are synthetic model tests, not original runtime or production Rust tests.
- Relative links/privacy reviewed; git diff --check passed.
- No Cargo gates rerun: documentation/synthetic research only, no Rust changes.
- Comparison plan unexecuted; normal speed 1/unpaused baseline and separate
  wall/game timestamps required. Pause/timescale controls remain testing leads.

FIDELITY STATUS:
INVESTIGATING

UNRESOLVED:
- Native trigger/thread timing, callback/score intermission race, overlay/FFOTD,
  total processing duration, timer/pickup tie and pre-ready disconnect cleanup.
- Destructive power dispatch versus Origins gate loss, recapture during pending
  shutdown, native grant/weapon/ammo options and at-capacity replacement.
- Sale/attachment/persistent price modes and damage/life-state consequences.

RECOMMENDED NEXT WORK:
- Original normal MP40 return versus ready expiry comparison, then controlled
  generator-loss shutdown interleaving with speed/pause/time provenance.

SCOPE NOT IMPLEMENTED:
- Production gameplay, weapon damage/firing, Box draw, perk/life persistence,
  movement/rendering, capture framework, runtime inputs, commits or pushes.
