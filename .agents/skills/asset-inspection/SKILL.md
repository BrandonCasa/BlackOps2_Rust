---
name: asset-inspection
description: Locate or catalog a bounded set of external licensed BO2 assets, investigate one format, or build a read-only inspector from confirmed format evidence. Use for asset access and metadata, not gameplay inference or wholesale extraction.
---

# Asset inspection

## Bounded task

Choose one objective: catalog a specified source set, investigate one format, or
implement one read-only inspector. State permitted roots/input types, allowed
output, resource limits and completion condition. Read
[project rules](../../../AGENTS.md), existing asset findings/configuration,
[architecture](../../../docs/architecture/README.md) and previous handoff.
Do not spawn agents, commit or expand into conversion or asset redistribution.

## Investigation

1. Resolve licensed sources through ignored `config/local_assets.toml`; tracked
   examples contain placeholders. When locating data, record real paths only
   there and keep tracked catalogs path-independent.
2. Start with metadata and appropriate hashes/identifiers. Document exactly which
   files/bytes a tool reads and which permitted summaries it emits.
3. For one format, establish signatures, version distinctions, bounds, units and
   references from observed samples. Label hypotheses; do not guess unknown fields
   or silently decode a different version with a superficially similar layout.
4. Check shared format/access abstractions and relevant Bevy asset facilities
   before proposing a loader adapter. Keep map semantics above reusable access.
5. Keep originals, extracted content and raw inspection output external. Review
   every repository candidate; an ignored directory is not permission to copy.

## Required evidence

Record source/build identifiers, permitted catalog metadata, format observations,
read/emit procedure, sample coverage, errors and unknowns in
`docs/reverse_engineering/`. Keep proprietary bytes, scripts, filenames containing
sensitive local paths and generated dumps out of tracked findings and fixtures.

## Implementation expectations

Assigned tools are read-only, safe Rust with explicit I/O/format errors, checked
offsets/lengths and bounded allocations. Reject unsupported versions and invalid
references without panics. Never embed originals or reconstruct an entire engine.
Loader integration requires its own assigned scope and supporting format evidence.

## Validation

Test synthetic valid, truncated, corrupt, oversized and unsupported inputs that
exercise observed format rules. Check path handling, output redaction and absence
of input mutation. Re-run the documented inspection on external samples where
available and report coverage; run applicable workspace checks for code changes.
Parser success alone establishes neither asset completeness nor rendering fidelity.

## Handoff and escalation

Save and return the [required handoff](../../../AGENTS.md#required-handoff),
including read/emit boundaries and actual checks. Use `UNKNOWN`/`INVESTIGATING`
without comparison evidence; never invent `MATCHED` or `VERIFIED`. Report missing
licensed data, unsupported/encrypted formats, risky output content or a requirement
to mutate originals to the outer agent. Do not invent format support to finish.
