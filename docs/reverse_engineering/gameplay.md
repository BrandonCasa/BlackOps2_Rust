# Gameplay foundation and retained observations

Source inspection established separate pending-spawn and living-enemy counts. Successful admission consumes pending budget; failed admission does not. A normal round completes only when both counts are exhausted. Ordinary population capacity is 24. Special enemies, map holds and replacement rules remain roadmap work.

The practical solo budget uses 24 + trunc(3 * multiplier), multiplier=max(1,floor(round/5)), additionally multiplied by round*0.15 from round 10. Rounds 1–5 apply factors 0.25,0.30,0.50,0.70,0.90. Round 1 starts with six enemies. This is retained practical reconstruction, not proof of every original callback and scaling detail.

Prior solo observations support Round 1, 500 points and pistol ammunition 8/32. Retained life-state code distinguishes standing/downed/terminal humans and alive/dead ordinary actors. Positive ordinary damage subtracts health; terminal repeat damage is ignored. Dead actor numeric health remains unknown. Revival/bleedout code is retained for future cooperative integration, without claiming cooperative play exists.

The playable profile uses player health 100, pistol damage 50, melee damage 150, first-round enemy health 150, practical health scaling, enemy attacks, direct/waypoint pursuit and fixed-step timing. Speeds, dimensions, gravity and weapon timings live together in `src/origins.rs` and may be overridden in the external game definition. These values and Avian movement are APPROXIMATE; native numerical motor/shader parity does not block gameplay implementation.

Controls preserve press edges until a fixed update consumes them, including press/release between ticks. Pause/focus loss clears held actions. Restart clears player/enemies/session and restores the map spawn, Round 1 and initial ammunition. Tests use synthetic geometry/data; real-asset verification is a separate opt-in invocation of the same executable.
