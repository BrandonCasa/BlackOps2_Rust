# Origins

Rebuild the complete Black Ops II Zombies Origins experience in stable Rust and Bevy. Prioritize playable systems and maintainable code; practical reconstruction is welcome. Be clear about approximations and unfinished features.

Use your judgment to choose tools, validation, and implementation strategy. Subagents and parallel work are welcome when useful; give them distinct ownership and preserve concurrent changes.

Keep one Cargo package and one game executable, `origins.exe`. Build on Bevy and Avian rather than duplicating engine infrastructure. Retain safe Rust and the locked dependency versions unless the task calls for changing them.

Keep `todo.md` as the sole project checklist, with concrete implementation tasks. Do not add codebase documentation or explanatory code comments unless requested. Leave rendering and graphics work out of the checklist.

Preserve original source assets. Keep prepared runtime assets under `assets/origins/`, validate their dependency closure when changing the pack, and use Git LFS for substantial binaries. Keep secrets, machine-specific paths, and generated builds out of commits.

Use validation proportional to the change. Development builds use `dev-dynamic`; release builds omit it. Use release builds for performance comparisons and development builds for diagnostics. Coordinate actual-game runs so only one owns the validation slot; never stop another task's process.

Review your changes, preserve unrelated work, and make coherent local commits with a clear summary and useful body. Push only when requested. Report what was actually checked and any remaining limitations.
