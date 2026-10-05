# Capture startup protection controls

This is a reviewed and accepted **PARTIAL fixture qualification**, supported by
[findings 0015](../reverse_engineering/0015-windows-fixture-protection-qualification.md).
It defines a bounded external qualification protocol, not a production sandbox,
Rust wrapper or game abstraction. Bevy keeps engine ownership; all seven Rust
crates and their unsafe policies remain unchanged. No SDK or original runtime is
admitted by this record.

The relevant source requirements remain [startup conditions 0012](../reverse_engineering/0012-sdk-capture-startup-write-conditions.md)
and [Windows edges 0013](../reverse_engineering/0013-sdk-windows-startup-edge-closure.md):
configuration refresh by the standard replay-marked child, dirty D3D11 cache writes
at device destruction, directory creation even during configuration reads, early
library/search-directory identity, TEMP logging/dump parents and independently
bounded child lifetime. They require controls before the corresponding effects,
with protection held until terminal cleanup. Synthetic controls do not establish
the SDK's actual path resolution or library behavior.

## Observed protocol

1. Parse ignored local TOML explicitly as UTF-8 with `tomllib`. Resolve the external
   inspection root, reject workspace containment, create a fresh task area and
   verify every owned fixture path. Keep source, binaries, paths, SIDs, detailed
   identities and diagnostics external. Preserve failures and prior frozen evidence.
2. Build only the self-authored ordinary-name I386/PE32 console fixture and its
   no-effect DLL with installed public Windows development facilities. No capture
   SDK binary, original data or profile is read. Isolate native calls in external
   C++/CPython qualification helpers. This is not an unsafe-policy exception in Rust.
3. Hold exact executable, DLL, configuration and cache handles with read access
   and `FILE_SHARE_READ`. Reject reparse leaves; compare canonical paths and
   volume/file IDs. Before process creation and again before release, hash bounded
   bytes from the same handle, then check final metadata. Record byte length,
   SHA-256, write time, attributes and source/build hashes. Hold directory handles
   separately for identity observation; they do not freeze directory contents or
   names on this host.
4. Create one unnamed non-inheritable job with kill-on-last-handle-close enabled.
   The helper also requests an active-process limit of two, but that limit is
   **not qualified**: deadline accounting reported four active entries. Do not
   infer an exact process inventory from those accounting counters. Installed-header
   ABI/offset/constant assertions compile for x64/x86, and an empty job queries back
   flags0x2008/limit2 with correct buffer lengths. No historical query-back exists;
   this audit does not explain the discrepancy or attribute it to Windows.
5. Create the parent with `CREATE_SUSPENDED | CREATE_NO_WINDOW`, explicit working
   directory, an explicit environment block setting TMP and TEMP to the same
   existing owned directory, and an exact three-handle standard-pipe inheritance
   list. Guard/job handles are non-inheritable. Assign and verify the exact job
   before resuming the initial thread. Compare process image path and creation
   time against the held image; fail closed on mismatch and clean the suspended
   process. A deliberate hash mismatch actually exercised this rejection.
6. The self-authored fixture reports its own process, current directory, resolved
   TEMP, loaded user-created DLL and config/cache identities, using opened file
   handles. The DLL is loaded by absolute path with DLL-directory/system32 search
   flags; its actual module path, file ID and exported value must match. The
   controller checks these records before sending the body gate. This gate is
   authored fixture cooperation, not an established SDK pre-init callback.
7. For the one fixture-defined child, create suspended with null environment and
   handle inheritance disabled, using the explicit working directory. The native
   parent checks image identity and job membership, then waits. The controller
   acquires the exact child process handle, matches creation time/image path and
   verifies membership in its particular job before authorizing child resume.
   Rehash held EXE/DLL identities at this child gate. Child-side observations then
   confirm inherited TMP/TEMP and matching file/directory/module identities.
8. For file protection, keep handles through all measured work and teardown.
   For directory creation denial, apply a separate DACL only to newly created,
   verified owned fixtures. The config directory denies addition of files and
   directories and deletion of children, without inheriting that deny into its
   share-guarded files. The separate strict directory inherits write/delete denies
   into its existing synthetic file. Record actual API errors, not presumed denial.
   Restore only these owned DACLs to their declared fixture baseline after the
   family is terminal; never alter shared/profile permissions.
9. Monitor independently of the native body, with a monotonic deadline, bounded
   output, bounded record count and fixed pipe readers. The deadline case closes
   the last job handle while the exact parent/child handles are confirmed live;
   wait on both handles and join readers before releasing file guards. Normal
   cases also require zero active job accounting before job closure. There is no
   post-close job accounting receipt for kill-on-close cases, and no complete
   explanation of the additional accounting entries.
10. Rehash protected files on the original handles and reopen paths before closing
    guards. Compare exact bytes/hash/length/write time/file ID/attributes, not
    merely filenames. Keep renamed directories and unexpected fixture contents
    as evidence. Directory write time is not restored. Case 2 failed its directory
    path predicate after file checks; its later receipt proves reopened preservation
    and ACL restoration, but does not supply a missing serialized end-handle receipt.

## Protection boundaries

| Control | Measured result | Practical limit |
| --- | --- | --- |
| Held read handles, READ sharing | Config/cache reads succeed; truncate, overwrite, delete and rename each fail with error32 | Exact files only; metadata/hash alone would not deny writes; no allocation or system-wide write bound |
| Attribute directory handle, READ/WRITE sharing without DELETE | New child file creation/write and same-volume directory rename succeed | Identity anchor is observation, not directory mutation enforcement |
| Owned directory DACL | Config `.tmp`, new strict file/subdirectory, existing unheld file write/delete fail with error5 | Only tested masks, owned paths and ordinary token access; owner/privilege/ACL changes and arbitrary path routing remain outside scope |
| Suspended process plus exact job/image admission | Wrong image hash rejected before initial thread resume; no body output/marker | Process identity does not deny filesystem writes; OS loader/CRT leaves are untraced |
| Explicit pre-creation TMP/TEMP | Actual parent/child `GetTempPathW` and directory IDs match | Environment routing does not redirect KnownFolder configuration/cache or prevent other paths |
| Loaded user-created DLL identity | Actual module path/file ID/export match held launch-time artifact | No real SDK, DbgHelp/dependency inventory or all-module closure |
| Non-inheritable kill-on-close job | Exact live parent and child become signaled; pipe EOF/readers/handles finish | Active-limit/accounting discrepancy unresolved; remaining job inventory cannot be queried after last close |

Microsoft documents [file sharing](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilew),
[file identity](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-getfileinformationbyhandle),
[process creation and inheritance](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-createprocessw),
[job ownership and kill-on-close](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects),
[assignment](https://learn.microsoft.com/en-us/windows/win32/api/jobapi2/nf-jobapi2-assignprocesstojobobject),
[job limit structure](https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-jobobject_extended_limit_information),
[TEMP selection](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-gettemppathw),
[file security](https://learn.microsoft.com/en-us/windows/win32/fileio/file-security-and-access-rights),
[handle-based DACL changes](https://learn.microsoft.com/en-us/windows/win32/api/aclapi/nf-aclapi-setsecurityinfo)
and [module loading](https://learn.microsoft.com/en-us/windows/win32/api/libloaderapi/nf-libloaderapi-loadlibraryexw).
Ten bounded page snapshots and exact URL/byte/SHA receipts remain external; these
API contracts are separate from the host-specific outcomes.

## Next admission

First resolve the unexpected owned-job accounting with one separately bounded
PID-list/creation-time discriminator, and qualify a directory-name control if the
later route requires one. Do not repeat task0021's four cases automatically.
Then outer-review a concrete ordinary-name x86 SDK startup/device/teardown design
that can establish the actual target and standard child resolver identities,
guarded config/cache/parent paths, early loaded library/CWD identities, registration
success versus fallback and cleanup. The tested cooperation gate and owned DACLs
are not authorization to apply profile ACLs or evidence of a supported pre-init
SDK path override. Windows/library effects remain untraced leaves; they are not
an invented universal no-write requirement or a full-goal blocker. Any SDK or
original runtime remains separately scoped and admitted.
