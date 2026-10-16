# Watchable Origins Easter egg run

```powershell
python tools/ee_validator/run.py --build --until foundation
```

The runner defaults to 2x gameplay speed. Use `--speed 10` for 10x gameplay,
or select any whole-number speed from 1 through 10. Use `--speed 1` for normal speed.
Single stepping still advances one 1/60-second tick, including native client
physics, and the ending movie always plays at 1x.
Validator launches skip the loading intro movie.
Walking holds forward continuously and steers toward each waypoint until arrival.
Blocked paths and simulation deadlines release the held controls and fail the run.
Combat queues trigger presses and releases at the equipped weapon's native fire
interval in simulation ticks, with no network round trip between shots.

The development-only bridge uses the normal input owner and physics. A normal
launch does not install it; builds without `dev-tools` exclude it.

The foundation collects a shovel through interaction, shoots an ordinary zombie,
saves screenshots, exercises pause/resume and one-tick stepping, and cancels held
input. It then terminates an actual controlling Python runner and checks that the
game independently releases input, records failure, and pauses. Recovery is explicit.

Control the scene from another terminal:

```powershell
python tools/ee_validator/control.py pause
python tools/ee_validator/control.py step
python tools/ee_validator/control.py resume
python tools/ee_validator/control.py abort
```

`step` advances one gameplay tick and its ordinary frame's physics. Escape pauses
or resumes a healthy run; F12 aborts permanently. Failure keeps the scene paused
for inspection and writes `game-failure.json`. Completing a checkpoint releases
the runner and leaves the game paused.

Reports contain gameplay observations, command completion, screenshots and the
failure reason. Command completion alone does not prove an objective: route
functions check actual gameplay results. There are no source-file gates, hashes,
code-change checks or provenance manifests.

The foundation passed visibly with seed 2818 at 2x speed in
`reports/ee-validator/foundation-20261016-f/summary.json`. Later milestones
extend the same fresh-match prefix. Assisted playthrough
success does not establish survival balance, economy or performance.

`--until generators-tank` adds generator activation and defense, movement onto
the tank, an ordinary paid ride, and the first lightning-part jump and pickup.
The complete fresh-match prefix passed at 10x with seed 2818 in
`reports/ee-validator/generators-20261016-al/summary.json`, including a recorded
native jump from the tank, landing on the near bank, the narrow walking path,
and player-owned part pickup.

With a validator scene paused, inspect first-jump launch combinations through
the read-only native collision query:

```powershell
python tools/ee_validator/probe_jumps.py --output reports/ee-validator/jump-probes.json
```

The query reuses the player's movement collision routine against the frozen
world and excludes the tank by default. Individual BRP candidates can include
`include_tank: true` and `runup_ticks` to inspect ground contact and native sprint
acceleration before jumping from the current vehicle pose. The result contains
each run-up tick and the accepted takeoff, if any. The vehicle remains frozen.
It selects candidates for a real input run;
probe landings cannot complete a milestone or collect a part.
