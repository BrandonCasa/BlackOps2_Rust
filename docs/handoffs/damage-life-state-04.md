TASK:
Resolve native reviveplayer current-health restoration for ordinary Origins revive.

STATUS:
COMPLETE
Qualified native rule resolved for observed Plutonium process; runtime comparison
and Steam-equivalence remain explicitly unverified.

FINDINGS:
- Runtime reviveplayer copies configured client maximum to entity current/max.
- Subsequent native setmaxhealth sets its integer argument to BOTH current/max.
  Thus source health-reboot100 supports completed unperked/no-bonus revive100/100.
- Disabled script clamping does not prevent native current restoration.
- Runtime RVAs differ from Steam; registry name/ID/function relationship was
  rediscovered and executable page spans checked rather than assumed.

EVIDENCE:
- Licensed t6zm.exe length12543488/hash and build24784239; unchanged afterward.
- Existing process identity/creation time and mapped PE header; outer UI r5354
  qualification is separate from Steam source.
- Three exact native spans repeated unchanged, selected registry entries/names,
  PAGE_EXECUTE_READ/MEM_IMAGE admission and complete x86 decoding.
- See ../reverse_engineering/damage-life-state-04.md and external task04 receipts.

CHANGES:
- Findings and this handoff only. No gameplay, Cargo, core or config changes.
- Tiny inspection helpers and cached Capstone dependency remain external; no
  full decompiler, memory dump, original content or local paths tracked.

VALIDATION:
- Synthetic valid/truncated/invalid-pointer registration checks pass.
- Runtime name/entry/function/page/repeat-span identities pass; original preserved.
- Permitted-content/path review and git diff --check pass.
- No original UI input, builtin execution, revive comparison or player-health read.
  Cargo unchanged and not rerun.

FIDELITY STATUS:
INVESTIGATING

UNRESOLVED:
- Live coop completion/cancellation and native/source agreement; loaded runtime
  script overrides and Steam-native equivalence.
- Unnamed native flag/helper effects, same-frame termination ordering.

RECOMMENDED NEXT WORK:
- Implement Origins resolved completion restoring100/100 under documented scope,
  rejecting stale downed episodes; retain live comparison requirement.
- Qualify controlled original comparison using desktop Plutonium and narrow
  observer leads if numeric state is required.

SCOPE NOT IMPLEMENTED:
- Gameplay/UI, actual health observer, original injection/mutation, bleedout,
  damage arithmetic, interaction, broad native engine model, commits or pushes.
