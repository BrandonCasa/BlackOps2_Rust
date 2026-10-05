# Local asset availability catalog

Recorded: 2026-10-04. Scope: locate a Steam-registered BO2 installation and
catalog external file metadata sufficiently to select one format investigation.
Data availability is **INVESTIGATING**; gameplay and visual fidelity are
**UNKNOWN**. No game was launched and no asset was decoded or extracted.

## Source identification and coverage

Steam registry metadata identified the active Steam root. Both Steam library
metadata files agreed on four registered libraries. App manifests in all four
accessible libraries were checked for the BO2 name; only one library contained
matching manifests. The base and Zombies manifests name the same installation
directory, whose actual files include `t6zm.exe` and the `zm_tomb` families below.
There were no additional BO2 manifest-backed candidates in those libraries.
Unregistered installations and other launchers were not searched once this source
was confirmed; this is not an exhaustive machine-wide inventory.

| Observed identifier | Value |
| --- | --- |
| Base app | `202970`, Call of Duty: Black Ops II |
| Base manifest build / state flags | `65428` / `68` |
| Zombies app | `212910`, Call of Duty: Black Ops II - Zombies |
| Zombies manifest build / target build / state flags | `24784239` / `24784239` / `4` |
| Zombies manifest language | `english` in user and mounted configuration |
| Zone language directories | `all`, `english` |
| Zombies executable size | 12,543,488 bytes |
| Executable resource metadata | file version `1.0`, product version `6`, product name Call of Duty(R): Black Ops II |
| Filesystem coverage | 309 files, 9,268,005,069 bytes; no reparse points observed |

These are local metadata observations, not a live game build measurement, proof
of ownership, a Steam integrity verification, or an interpretation of state flags.
The installation is selected under the user's licensed-originals task context.
Executable resource versions are not substitutes for the manifest build or a
runtime version. Windows executable/container layout is present; executable
architecture and other platforms were not investigated. Shared depots and
additional files make manifest per-app sizes unsuitable as whole-directory totals.

## Catalog measurements

All sizes are exact file-length bytes at inspection time, not expanded size or
allocated disk space. Counts include local ancillary files; `players` contents
were not read. Extension names describe on-disk families, not approved formats.

| Top-level family | Files | Bytes |
| --- | ---: | ---: |
| Root files | 8 | 12,986,342 |
| `players` | 7 | 10,503 |
| `redist` | 159 | 107,926,405 |
| `sound` | 48 | 4,267,808,768 |
| `video` | 8 | 256,734,382 |
| `zone` | 79 | 4,622,538,669 |

| Extension family | Files | Bytes |
| --- | ---: | ---: |
| `.ff` | 66 | 558,722,752 |
| `.ipak` | 12 | 4,009,885,696 |
| `.sabl` | 18 | 737,970,176 |
| `.sabs` | 30 | 3,529,838,592 |
| `.webm` | 7 | 255,542,930 |
| `.bik` | 1 | 1,191,452 |
| Other metadata, binaries and ancillary files | 175 | 174,853,471 |

The other extensions are `.bdg`, `.bmp`, `.cab`, `.cgp`, `.chp`, `.clump`,
`.dll`, `.exe`, `.inf`, `.ini`, `.txt`, and `.vdf`. Their semantics and origin
were not investigated. The `zone` family includes one `.clump` in addition to
the `.ff` and `.ipak` totals.

## Origins candidate identifiers

| Relative source identifier | Bytes |
| --- | ---: |
| `zone/all/zm_tomb.ff` | 78,753,344 |
| `zone/all/zm_tomb_patch.ff` | 1,288,384 |
| `zone/english/en_zm_tomb.ff` | 74,176 |
| `sound/zmb_tomb.all.sabl` | 103,546,880 |
| `sound/zmb_tomb.all.sabs` | 481,327,104 |
| `sound/zmb_tomb.english.sabs` | 393,840,640 |
| `sound/zmb_tomb_load.all.sabs` | 29,353,984 |
| `sound/zmb_tomb_load.english.sabs` | 12,253,184 |
| `video/zm_tomb_load.webm` | 68,728,988 |
| `video/zm_tomb_ee.webm` | 25,234,901 |
| `zone/all/dlczm4.ipak` | 407,896,064 |

Observation: the zone, sound and video identifiers share the `tomb` token, with
base, patch, loading and language-labelled variants. **Inference:** these form
the Origins investigation candidate set. Filename labels alone do not demonstrate
map identity, patch order, loading requirements or actual contents. `dlczm4.ipak`
is a DLC-labelled candidate; its association with Origins is unconfirmed.

Other `.ipak` identifiers include `base`, `zm`, `patch_zm`, `dlczm0` through
`dlczm3`, `dlczm0_load_zm`, `zm_nuked`, `zm_nuked_patch`, and `en_base`.
These indicate possible shared/base/map families worth checking later, without
establishing references or dependency order.

## Bounded prefix observations and sample identity

Seven samples were read from offset zero for 16 bytes each; three of those were
reread for another 16-byte field observation. None exceeded 32 interpreted bytes
per sample. No body parsing, decompression, decryption, conversion or extraction
occurred. Full-file streaming SHA-256 reads covered those seven samples and
`t6zm.exe`; hashing emitted digests only. Executable version-resource metadata
was queried, without disassembly or execution.

| Sample coverage | Observed prefix metadata | Limits |
| --- | --- | --- |
| Three `zm_tomb` `.ff` files | First eight ASCII bytes `TAff0100`; little-endian u32 at offset 8 is `147` in all three | `147` is a version candidate, not an established field definition; remaining structure unknown |
| `dlczm4.ipak` | First four ASCII bytes `KAPI`; little-endian u32 at offset 4 is `327680`, at offset 8 is `407896064` | Offset 8 equals this file's length; field meaning/version encoding remains unconfirmed |
| `zmb_tomb.all.sabl`, `zmb_tomb.all.sabs` | First four ASCII bytes `2UX#`; little-endian u32 at offset 4 is `14`, at offset 8 is `20` | Shared signature/values observed, but streamed/loaded semantics and field definitions unconfirmed |
| `zm_tomb_load.webm` | First four bytes have hexadecimal signature `1A45DFA3` | No media/container decoding or playback |

Representative SHA-256 digests establish reproducible sample identity:

| Relative identifier | SHA-256 |
| --- | --- |
| `t6zm.exe` | `f6f7104af2bd0c2b931ea1e739969e85845e499affbf686144c6b490b344ab1e` |
| `zone/all/zm_tomb.ff` | `663c2f6e275fb6a8a2844bcc0e31808abe681ce7c2a2ddc6f616ef98fc7cd0b5` |
| `zone/all/zm_tomb_patch.ff` | `770d8a2c3558c9e4c981d8d6db6bd67698111f01e8be32423c66b71719b4c2ca` |
| `zone/english/en_zm_tomb.ff` | `fe779a385fdb6626b74bb00fc5fccd3f086e645f0cf2f35be00f032d67cf73b8` |
| `zone/all/dlczm4.ipak` | `e38e3b2f90b79642cf8e64b79e8334f98e9d3a2d5528e4aa414d40023db90dec` |
| `sound/zmb_tomb.all.sabl` | `140cc1fb8d86d42058898125a46a25b8f0b13f6eb5edb5f7072cff097aba04d1` |
| `sound/zmb_tomb.all.sabs` | `cf5f9820b10c81b7e14c3ef0297b7009b0bd79cd607db1f8e1a05988aae5389a` |
| `video/zm_tomb_load.webm` | `94b416373db73e04bb71ffe5444d94c437af4030ef25d777716341b9c4d97855` |

## Reproduction and output boundary

1. Read the Steam registry installation pointer and its `libraryfolders.vdf`
   metadata; inspect app manifest names/BO2 identifiers across registered libraries.
2. Resolve the matching manifest installation directory and verify actual Zombies
   executable plus `tomb`-labelled files. Store both absolute external roots only
   in ignored `config/local_assets.toml`, following the tracked example schema.
3. Recursively enumerate file metadata only within that installation; group counts
   and lengths by top-level directory/extension. Check reparse-point absence.
   Do not read local player data, account content or other game installations.
4. Open the listed samples with read-only streams for bounded prefixes, hash the
   eight samples, and query executable resource metadata. Record offset/endian
   observations without interpreting unknown fields.
5. Keep local summary `0001-local-asset-catalog.json` in the configured external
   inspection directory. It contains aggregate metadata, relative sample
   identifiers, digests, timestamps and bounded field observations; no original
   file bodies, player data, scripts or extracted content. Only this permitted,
   path-independent findings document belongs in Git.
6. Parse actual/example TOML and compare keys; verify absolute existing roots stay
   outside the repository and the actual configuration is ignored. Rehash all
   samples and compare lengths/write timestamps to confirm no observed mutation.

No source write was performed. The eight sample sizes, last-write timestamps and
SHA-256 digests were unchanged on repeated verification. This establishes the
sample mutation check, not all-file Steam integrity. The diagnostic Bevy boot does
not consume the configuration; no asset-access API has been implemented.

## One recommended next investigation

Investigate the **`TAff0100` / candidate-147 `.ff` envelope metadata** on the three
listed `zm_tomb` samples. Establish the header field definitions, version/support
boundaries, structural length checks and base/patch/localized distinctions using
bounded reads and authoritative format evidence before designing an inspector.
Escalate encrypted/unsupported sections without guessing or extracting them.

This family has a confirmed base, patch and localized candidate, and can provide
evidence for reusable asset access before map semantics. Shared prefix structure
is an abstraction lead, not an approved BO2 asset/entity API. Body/entity schemas,
asset references, actual Origins identity, data completeness and every gameplay
or visual comparison remain unresolved.
