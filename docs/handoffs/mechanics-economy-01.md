TASK:
Establish one representative Origins points/purchase eligibility/debit/acquisition
boundary, with generator, persistence, Box and Pack-a-Punch dependencies bounded.

STATUS:
COMPLETE

FINDINGS:
- Juggernog's nominal 2500-point machine purchase has ordered player/ownership,
  Origins-zone, affordability and capacity admission; payment precedes bottle grant.
- Origins directly associates that machine with generator_nml_right and tests
  current player control. Global power, machine hint and owned perk state differ.
- Debit mirrors persistent score and emits spent-points; paused ownership rejects
  duplicates. Effective-price and capacity callbacks cannot be silently omitted.
- Box zone locks and Pack-a-Punch all-zone/on-use shutdown deferral are dependency
  observations only. Power-loss races and original runtime timing remain unknown.

EVIDENCE:
- Read project state, architecture, origins-analysis and reverse-engineering skills,
  directory evidence; inspected retained build24784239 compiled patch/map scripts.
- Concrete original compiled hashes, read/emit procedure and named function anchors
  in [purchase-boundary](../reverse_engineering/origins-mechanics/economy/purchase-boundary.md).
- Reused licensed decompiled/disassembled perk views; generated bounded score,
  capture-zone, magicbox and utility views externally, plus two disassemblies.
- No original-game run, behavioral measurement or gameplay comparison performed.

CHANGES:
- Added economy purchase-boundary record, synthetic-purchase-cases.json and a
  user-created check_synthetic_purchase.py beside it for reproducible model checks.
- External mechanics-economy-01 retains tool views, identity receipt, synthetic
  checker/results; no original text, asset, dump or machine path added to Git.
- Proposed staged reusable transaction/eligibility data, not an approved API.

VALIDATION:
- All six per-file decoder invocations exited0; critical perk ordering, score
  mutation, zone validator and acquisition timeout cross-checked in disassembly.
- Source/tool hashes checked; container identities agree with retained receipts.
- `python docs/reverse_engineering/origins-mechanics/economy/check_synthetic_purchase.py`
  exits0 and reports PASS, case_count16. These synthetic abstract model scenarios
  check meaningful balance, count, delayed
  ownership, denial-priority and repeat checks. These do not test production Rust.
- Relative links and tracked privacy reviewed; git diff --check passed.
- Cargo gates not rerun: documentation/synthetic research only; no Rust changes.
- Original comparison cases documented but unexecuted; zero fidelity samples.

FIDELITY STATUS:
INVESTIGATING

UNRESOLVED:
- Overlay/FFOTD/native execution, exact animation duration, same-frame ordering,
  modifier availability/formula, cap overrides and power-loss acquisition race.
- Complete round/session persistence and global refund policy unestablished;
  damage/downing/revival belongs to the separate life-state investigation.

RECOMMENDED NEXT WORK:
- Trace one Pack-a-Punch payment-to-return transition, especially power-loss
  shutdown deferral, then execute discriminating original comparisons when allowed.

SCOPE NOT IMPLEMENTED:
- Gameplay/engine code, all economy systems, generator activation, Box draw,
  Pack-a-Punch implementation, captures, runtime inputs, commits or pushes.
