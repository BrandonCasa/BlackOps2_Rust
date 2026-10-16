# Origins Easter egg validator

Development-only BRP and a visible Python runner. Ordinary launches do not install
the bridge. Builds without `dev-tools` exclude it.

Run the foundation increment:

```powershell
python tools/ee_validator/run.py --build --until foundation
```

The runner loads an Original solo match, enables assistance, collects a shovel
through interaction, shoots an ordinary zombie, saves captures, exercises
pause/resume and one-tick stepping, cancels a held action, and kills its separate
Python heartbeat process to prove game-side recovery. A failure retains the scene
and writes `game-failure.json` independently of Python.

Control the visible session from another terminal:

```powershell
python tools/ee_validator/control.py pause
python tools/ee_validator/control.py step
python tools/ee_validator/control.py resume
python tools/ee_validator/control.py abort
```

`step` advances one gameplay tick and the ordinary frame's physics before pausing.
Escape pauses/resumes a healthy run; F12 aborts. Reconnecting never automatically
resumes a failed run. Abort cannot be resumed. `quit` closes the owned game.

Reports contain command receipts, gameplay events, source references, captures,
and observed failures. No file hashes or source-change checks are performed.
Accepted commands are not passes: objective contracts require observed outcomes.

Route increments beyond the foundation are added only after its visible checks
pass. Completing an assisted increment does not validate normal spawning, economy,
survival balance, unscripted timing, or performance.
