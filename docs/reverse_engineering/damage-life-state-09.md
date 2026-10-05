# Fresh health-observer diagnostic run

Recorded 2026-10-05. **INVESTIGATING**; no damage comparison pass. Review began
at clean c99beed with state/architecture/API/tests and handoff08. Investigation
and gameplay-fidelity skills applied. Existing observer/profile/checks unchanged.

Parent admitted fresh Escape-paused r5354 LAN Origins solo: round1 Richtofen,
score500, ammo8/32; no purchases/combat/console/timescale input. Worker waited for
READY, started exactly one existing finite read-only sampler, then parent resumed
through Escape. Parent later observed results0kills/1down with no further gameplay
input. Exact attack/lethal frame and sample-clock alignment were not captured.
Worker performed no UI, writes, injection or additional native investigation.

Run external task08 `health-sample.py --pid 42012 --count 1000 --interval 0.05`;
retain numeric JSONL under task09. Exec session73783 terminated exit0, sampler_exit0;
no sampler remains. Existing process/header/code/registration boundary checks pass.
Helper/profile hashes match task08 receipts; runtime/source distinction remains
as qualified in [observer evidence](damage-life-state-08.md).

| Observable | Result |
| --- | --- |
| Records / acquisition span | 1000 /55.5391978seconds |
| Accepted / refused | 972 /28 |
| Accepted index/current/entity-max/client-max | Always0/100/100/100; no transition |
| Refusal reason | All28 unstable_fields:state |
| Flags/gate/health/entity/client changes reported | None |
| Health emitted on refusal | None |

`state` names the derived local-state **pointer** at qualified root+0x28, not Rust
LifeState, health, flags or sessionstate. Two snapshots disagreed on that pointer.
This proves the immediate refusal condition, not its engine mechanism. Rotating
buffers remain a hypothesis. Native admission tests bit0/mask6, but refusals did
not identify either flag word; no flag-mask fix is justified and checks stay intact.

Accepted100 does not establish no damage. An unused/stale server route, another
health-update field, missed short damage/reset interval or inadequate attack overlap
remain alternatives. The28 refusals do not explain unchanged accepted health.

External analyzer/identity receipts preserve transitions, exact refusal brackets
and hashes. JSONL SHA-256:
fc415d0ab48289cbb37d477f697b49e43769773ab30d8a10bd30d5d3bed51416.
Existing15 synthetic cases and5 field-name diagnostic cases pass; unchanged
accepts, changed flags/gate/current/root refuse without exposing values. Content/
whitespace checks pass. Cargo unchanged/not rerun; no automatic repeat.

Next separately trace registered finishplayerdamage, active runtime dispatch and
its directly necessary health writer: does actual player damage update the same
entity table/current slot? Name/table presence alone does not prove execution.
Avoid value scanning or broader schemas; qualify that route before another
outer-controlled isolated-hit comparison. Solo supplies no coop proof.
