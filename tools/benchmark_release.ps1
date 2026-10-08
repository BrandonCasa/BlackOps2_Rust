param(
    [Parameter(Mandatory = $true)][string]$Executable,
    [Parameter(Mandatory = $true)][string]$Output,
    [ValidateSet('debug', 'release')][string]$Profile = 'release',
    [ValidateSet('moving', 'combat')][string[]]$Workloads = @('moving', 'combat'),
    [ValidateRange(1, 9)][int]$Repeats = 3,
    [string]$Baseline
)

$ErrorActionPreference = 'Stop'
$repository = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$exe = [IO.Path]::GetFullPath($Executable)
$package = Split-Path -Parent $exe
$report = [IO.Path]::GetFullPath($Output)
foreach ($externalPath in @($package, $report)) {
    if ($externalPath.Equals($repository, [StringComparison]::OrdinalIgnoreCase) -or
        $externalPath.StartsWith($repository + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
        throw 'Preserve the release package and measurements outside the repository'
    }
}
if (-not (Test-Path -LiteralPath $exe -PathType Leaf)) { throw 'Missing packaged executable' }
if (-not (Test-Path -LiteralPath (Join-Path $package 'assets/origins/game.json'))) { throw 'Missing executable-relative pack' }
if (Test-Path -LiteralPath $report) { throw 'Use a fresh report directory; previous runs are never overwritten' }
New-Item -ItemType Directory -Path $report | Out-Null
$manifest = Get-ChildItem -LiteralPath (Join-Path $package 'assets/origins') -File -Recurse |
    Sort-Object FullName | ForEach-Object {
        [ordered]@{ path = [IO.Path]::GetRelativePath($package, $_.FullName); bytes = $_.Length;
            sha256 = (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant() }
    }
$manifest | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath (Join-Path $report 'asset-manifest.json')
[ordered]@{
    executable = $exe
    executable_sha256 = (Get-FileHash -LiteralPath $exe -Algorithm SHA256).Hash.ToLowerInvariant()
    asset_manifest_sha256 = (Get-FileHash -LiteralPath (Join-Path $report 'asset-manifest.json')).Hash.ToLowerInvariant()
    started_utc = [DateTime]::UtcNow.ToString('o')
    workloads = $Workloads
    repeats = $Repeats
    build_profile = $Profile
} | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $report 'identity.json')

$runs = [Collections.Generic.List[object]]::new()
foreach ($workload in $Workloads) {
    for ($index = 1; $index -le $Repeats; $index++) {
        while (Get-Process -Name origins, cargo, rustc -ErrorAction SilentlyContinue) {
            Write-Output 'Game or compiler active; waiting for release measurement resources'
            Start-Sleep -Seconds 10
        }
        $runFolder = Join-Path $report "$workload-$index"
        New-Item -ItemType Directory -Path $runFolder | Out-Null
        if ($runFolder.Contains('"')) { throw 'Invalid output path' }
        Write-Output "Starting $workload $index/$Repeats"
        $owned = Start-Process -FilePath $exe -WorkingDirectory $package -WindowStyle Hidden -PassThru `
            -ArgumentList @("--release-benchmark-$workload", ('"' + $runFolder + '"')) `
            -RedirectStandardOutput (Join-Path $runFolder 'game.log') `
            -RedirectStandardError (Join-Path $runFolder 'stderr.log')
        $ram = [Collections.Generic.List[object]]::new()
        $timer = [Diagnostics.Stopwatch]::StartNew()
        $collision = $false
        $buildInterference = $false
        try {
            while (-not $owned.HasExited) {
                $others = Get-Process -Name origins -ErrorAction SilentlyContinue | Where-Object Id -ne $owned.Id
                if (Get-Process -Name cargo, rustc -ErrorAction SilentlyContinue) {
                    $buildInterference = $true
                }
                if ($others -or $timer.Elapsed.TotalSeconds -gt 960) {
                    $collision = [bool]$others
                    # The Process object is the owned launch, never another chat's process.
                    $owned.Kill()
                    throw 'Owned benchmark stopped: validation collision or timeout; retain failed run evidence'
                }
                $owned.Refresh()
                if (-not $owned.HasExited) {
                    $ram.Add([ordered]@{ seconds = $timer.Elapsed.TotalSeconds;
                        working_set_bytes = $owned.WorkingSet64; private_bytes = $owned.PrivateMemorySize64 })
                }
                Start-Sleep -Milliseconds 500
            }
            $owned.WaitForExit()
            if ($owned.ExitCode -ne 0) { throw "Benchmark exited $($owned.ExitCode); inspect $runFolder" }
        } finally {
            $ram | ConvertTo-Json -Depth 3 | Set-Content -LiteralPath (Join-Path $runFolder 'process-memory.json')
            [ordered]@{ pid = $owned.Id; elapsed_seconds = $timer.Elapsed.TotalSeconds;
                collision = $collision; build_interference = $buildInterference } |
                ConvertTo-Json | Set-Content -LiteralPath (Join-Path $runFolder 'launch.json')
            $owned.Dispose()
        }
        if ($buildInterference) {
            throw "Concurrent compilation invalidated the release comparison; preserve $runFolder and repeat after builds are free"
        }
        $result = Get-Content -LiteralPath (Join-Path $runFolder 'performance.json') -Raw | ConvertFrom-Json
        if (-not $result.passed -or $result.profile_tracy -or $result.profile_chrome -or
            $result.build_profile -ne $Profile -or $result.logging_enabled -ne ($Profile -eq 'debug') -or
            $null -eq $result.render_diagnostics -or $result.render_diagnostics -or
            $null -eq $result.pipeline_cache_per_frame_inspection -or $result.pipeline_cache_per_frame_inspection -or
            $result.sample_wall_seconds -lt 30) {
            throw "Unqualified $Profile benchmark: $runFolder"
        }
        $runs.Add([ordered]@{ workload = $workload; run = $index; summary = $result.summary;
            build_profile = $result.build_profile; logging_enabled = $result.logging_enabled;
            ddgi_enabled = $result.shared_lighting.ddgi_enabled -ne $false;
            physical_resolution = $result.physical_resolution; adapter = $result.adapter;
            antialiasing = $result.antialiasing; preprocessing = $result.preprocessing;
            seed = $result.seed; controlled_setup = $result.controlled_setup;
            workload_revision = $result.workload_revision; start_counts = $result.start_counts;
            end_counts = $result.end_counts; living_min_max = $result.living_min_max })
        Write-Output "$workload $index complete: $($result.summary.fps) FPS, p95 $($result.summary.p95_ms) ms"
    }
}
function Get-Median($values) {
    $ordered = @($values | Sort-Object)
    $middle = [int][Math]::Floor($ordered.Count / 2)
    if ($ordered.Count % 2) { return $ordered[$middle] }
    return ($ordered[$middle - 1] + $ordered[$middle]) / 2
}
$groups = foreach ($workload in $Workloads) {
    $rows = @($runs | Where-Object { $_.workload -eq $workload })
    [ordered]@{ workload = $workload; median_p95_ms = Get-Median @($rows | ForEach-Object { $_.summary.p95_ms });
        median_fps = Get-Median @($rows | ForEach-Object { $_.summary.fps }) }
}
$comparison = @()
if ($Baseline) {
    $reference = Get-Content -LiteralPath (Join-Path $Baseline 'summary.json') -Raw | ConvertFrom-Json
    foreach ($group in $groups) {
        $before = $reference.workloads | Where-Object workload -eq $group.workload
        if (-not $before) { throw 'Baseline does not contain the measured workload' }
        $beforeRuns = @($reference.runs | Where-Object workload -eq $group.workload)
        $afterRuns = @($runs | Where-Object { $_.workload -eq $group.workload })
        if ($beforeRuns.Count -lt 3 -or $afterRuns.Count -lt 3) { throw 'Acceptance requires at least three runs per workload' }
        foreach ($row in $afterRuns) {
            # Historical baselines always enabled DDGI; reject mixed lighting work.
            $beforeDdgi = $beforeRuns[0].ddgi_enabled -ne $false
            if ($row.ddgi_enabled -ne $beforeDdgi) { throw 'Incomparable lighting workload: DDGI configuration changed' }
            foreach ($field in @('build_profile', 'logging_enabled', 'physical_resolution', 'adapter', 'workload_revision', 'antialiasing', 'preprocessing', 'seed', 'controlled_setup')) {
                $expected = $beforeRuns[0].$field | ConvertTo-Json -Depth 8 -Compress
                $actual = $row[$field] | ConvertTo-Json -Depth 8 -Compress
                if ($actual -ne $expected) { throw "Incomparable $($group.workload) workload: $field changed" }
            }
        }
        $comparison += [ordered]@{ workload = $group.workload;
            p95_ratio = $group.median_p95_ms / $before.median_p95_ms;
            fps_ratio = $group.median_fps / $before.median_fps;
            passed = ($group.median_p95_ms -le $before.median_p95_ms * 1.03 -and $group.median_fps -ge $before.median_fps * 0.97) }
    }
}
[ordered]@{ runs = $runs.ToArray(); workloads = @($groups); comparison = $comparison } |
    ConvertTo-Json -Depth 12 | Set-Content -LiteralPath (Join-Path $report 'summary.json')
if ($comparison | Where-Object { -not $_.passed }) { throw 'Release performance regression exceeds the accepted 3 percent margin' }
Write-Output (Join-Path $report 'summary.json')
