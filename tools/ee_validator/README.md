# Watchable Origins Easter egg run

```powershell
python tools/ee_validator/run.py --build --until foundation
```

The runner defaults to 2x gameplay speed. Use `--speed 1` for normal speed.
Single stepping still advances one 1/60-second tick, including native client
physics, and the ending movie always plays at 1x.

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

Only the foundation is implemented so far; visible validation remains pending.
Later milestones will extend the same fresh-match prefix. Assisted playthrough
success does not establish survival balance, economy or performance.
