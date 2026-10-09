param(
    [Parameter(Mandatory = $true)][string]$Output,
    [Parameter(Mandatory = $true)][string]$TracyDirectory,
    [string]$ExePath,
    [string]$AllocationExporter,
    [ValidateSet('default', 'direct', 'culling', 'cpu')][string]$Preprocessing = 'default',
    [int]$View = 8,
    [ValidateRange(1, 86400)][int]$CaptureSeconds = 300,
    [switch]$SkipBuild,
    [switch]$Memory,
    [switch]$NoTracy,
    [switch]$AllViews,
    [switch]$Moving,
    [switch]$Combat,
    [switch]$Generators
)
$ErrorActionPreference = 'Stop'
if (([int]$AllViews.IsPresent + [int]$Moving.IsPresent + [int]$Combat.IsPresent + [int]$Generators.IsPresent) -gt 1) {
    throw 'Choose one profiling workload'
}
$repository = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$buildProfile = if ($NoTracy) { 'release' } else { 'debug' }
$gameExecutable = if ($ExePath) { [IO.Path]::GetFullPath($ExePath) } else { Join-Path $repository "target/$buildProfile/origins.exe" }
if ($ExePath -and -not $SkipBuild) { throw 'An explicit preserved executable requires -SkipBuild' }
if ($NoTracy -and ($Memory -or $AllocationExporter)) { throw 'An ordinary executable comparison cannot request Tracy allocation instrumentation' }
if ($AllocationExporter -and -not (Test-Path -LiteralPath $AllocationExporter)) { throw 'Allocation exporter does not exist' }
$outputPath = [IO.Path]::GetFullPath($Output)
if ($outputPath.Equals($repository, [StringComparison]::OrdinalIgnoreCase) -or $outputPath.StartsWith($repository + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
    throw 'Profiling reports and captures must remain external'
}
if (Test-Path -LiteralPath $outputPath) { throw 'Use a fresh output directory to preserve earlier evidence' }
$capture = Join-Path $TracyDirectory 'tracy-capture.exe'
$export = Join-Path $TracyDirectory 'tracy-csvexport.exe'
if (-not (Test-Path -LiteralPath $capture) -or -not (Test-Path -LiteralPath $export)) {
    throw 'Supply the Tracy capture/export tools matching cargo tree --features profile-tracy -i tracy-client-sys'
}
$savedEnvironment = @{}
foreach ($key in @('NO_COLOR', 'RUST_LOG', 'TRACY_NO_SYS_TRACE', 'TRACY_SYMBOL_OFFLINE_RESOLVE', 'TRACY_ONLY_LOCALHOST')) {
    $savedEnvironment[$key] = [Environment]::GetEnvironmentVariable($key, 'Process')
}
Push-Location $repository
try {
    if (-not $SkipBuild) {
        if ($NoTracy) {
            $releaseFeatures = @()
            & cargo build --workspace --release --locked @releaseFeatures
        } else {
            $profilingFeature = if ($Memory) { 'profile-tracy-memory' } else { 'profile-tracy' }
            & cargo build --workspace --locked --features "dev-dynamic,$profilingFeature"
        }
        if ($LASTEXITCODE -ne 0) { throw "Build failed for $buildProfile measurements" }
    }
    if (Get-Process -Name origins -ErrorAction SilentlyContinue) {
        throw 'The shared actual-game validation slot is occupied; wait for that run to finish'
    }
    if (-not (Test-Path -LiteralPath $gameExecutable)) { throw 'Profiling game executable does not exist' }
    New-Item -ItemType Directory -Path $outputPath | Out-Null
    [ordered]@{
        executable = $gameExecutable
        executable_sha256 = (Get-FileHash -LiteralPath $gameExecutable -Algorithm SHA256).Hash.ToLowerInvariant()
        memory_instrumentation_requested = $Memory.IsPresent
        tracy_capture_requested = -not $NoTracy.IsPresent
        lighting_method = 'native_raster'
        preprocessing_requested = $Preprocessing
        workload = if ($Generators) { 'generators' } elseif ($Combat) { 'combat' } elseif ($Moving) { 'moving' } elseif ($AllViews) { 'all views' } else { 'steady' }
        view = $View
        capture_seconds = $CaptureSeconds
        started_utc = [DateTime]::UtcNow.ToString('o')
    } | ConvertTo-Json | Set-Content -LiteralPath "$outputPath/profiling-run.json" -Encoding utf8
    $env:NO_COLOR = '1'
    $env:RUST_LOG = 'info,wgpu=error,naga=warn'
    $env:TRACY_NO_SYS_TRACE = '1'
    $env:TRACY_SYMBOL_OFFLINE_RESOLVE = '1'
    $env:TRACY_ONLY_LOCALHOST = '1'
    if (-not $NoTracy) {
        $captureProcess = Start-Process $capture -ArgumentList @('-o', "`"$outputPath/capture.tracy`"", '-a', '127.0.0.1', '-s', $CaptureSeconds) -WindowStyle Hidden -RedirectStandardOutput "$outputPath/capture.log" -RedirectStandardError "$outputPath/capture-errors.log" -PassThru
    }
    $memoryObserver = Start-ThreadJob -ArgumentList $gameExecutable, $PID, $outputPath, $CaptureSeconds -ScriptBlock {
        param($executable, $ownerPid, $reportPath, $deadlineSeconds)
        $samples = [Collections.Generic.List[object]]::new()
        $ownedGame = $null
        $deadline = [DateTime]::UtcNow.AddSeconds($deadlineSeconds + 60)
        while ([DateTime]::UtcNow -lt $deadline) {
            if (-not $ownedGame) {
                foreach ($candidate in @(Get-Process -Name origins -ErrorAction SilentlyContinue)) {
                    if ($candidate.Path -ne $executable) { continue }
                    $identity = Get-CimInstance Win32_Process -Filter "ProcessId = $($candidate.Id)"
                    if ($identity.ParentProcessId -eq $ownerPid) { $ownedGame = $candidate; break }
                }
            }
            if ($ownedGame) {
                $ownedGame.Refresh()
                if ($ownedGame.HasExited) { break }
                $samples.Add([ordered]@{
                    utc = [DateTime]::UtcNow.ToString('o')
                    working_set_bytes = $ownedGame.WorkingSet64
                    private_bytes = $ownedGame.PrivateMemorySize64
                    total_cpu_seconds = $ownedGame.TotalProcessorTime.TotalSeconds
                    threads = $ownedGame.Threads.Count
                })
            }
            Start-Sleep -Milliseconds 1000
        }
        [ordered]@{
            owned_game_pid = if ($ownedGame) { $ownedGame.Id } else { $null }
            executable = $executable
            sampling_seconds = 1
            scope = 'Entire owned game process; startup and workload included. Working set and private bytes exclude physical GPU memory.'
            samples = $samples.ToArray()
        } | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath "$reportPath/process-memory.json" -Encoding utf8
    }
    $gameArguments = if ($Generators) { @('--verify-generators', $outputPath) } elseif ($Combat) { @('--benchmark-combat', $outputPath) } elseif ($AllViews) { @('--benchmark', $outputPath) } elseif ($Moving) { @('--benchmark-moving-view', $outputPath, $View) } else { @('--benchmark-view', $outputPath, $View) }
    $preprocessingArgument = switch ($Preprocessing) {
        'direct' { '--direct-preprocessing' }
        'culling' { '--gpu-culling' }
        'cpu' { '--cpu-preprocessing' }
    }
    if ($preprocessingArgument) { $gameArguments += $preprocessingArgument }
    & $gameExecutable @gameArguments 2> "$outputPath/game.log"
    if ($LASTEXITCODE -ne 0) { throw "Actual game benchmark failed; inspect $outputPath/game.log" }
    $memoryObserver | Wait-Job -Timeout 5 | Out-Null
    $memoryObserver | Receive-Job -ErrorAction Stop | Out-Null
    if ($memoryObserver.State -ne 'Completed') { throw 'Owned game RAM observer did not finish' }
    if (Test-Path -LiteralPath "$outputPath/failure.json") { throw "Actual game reported a failed workload; inspect $outputPath/failure.json" }
    $completionPath = if ($Generators) { "$outputPath/generators.json" } else { "$outputPath/verification.json" }
    if (-not (Test-Path -LiteralPath $completionPath) -or -not (Test-Path -LiteralPath "$outputPath/performance.json")) {
        throw 'Actual game exited without completion and performance receipts; no complete benchmark may be claimed'
    }
    $receipt = Get-Content -LiteralPath $completionPath -Raw | ConvertFrom-Json
    if ($receipt.passed -ne $true) { throw 'Actual game completion receipt did not pass' }
    $performance = Get-Content -LiteralPath "$outputPath/performance.json" -Raw | ConvertFrom-Json
    if ($performance.run_environment.continuous_window_updates -ne $true -or
        $performance.run_environment.display_sleep_inhibited -ne $true -or
        $performance.run_environment.system_sleep_inhibited -ne $true) {
        throw 'Profiling lacks active focus/display/system sleep protection; rebuild the executable'
    }
    if ($performance.build_profile -ne $buildProfile) {
        throw "Expected $buildProfile executable; measurement receipt reports a different profile"
    }
    $lighting = if ($performance.shared_lighting) { $performance.shared_lighting } else { $receipt.shared_lighting }
    if ($lighting.method -ne 'native_raster' -or $lighting.ray_tracing -ne $false) {
        throw 'Expected native raster lighting without ray tracing'
    }
    if ($NoTracy) {
        if ($performance.profile_tracy -ne $false -or $performance.profile_chrome -ne $false) {
            throw 'Ordinary executable comparison contains tracing features'
        }
        Write-Output "$outputPath/performance.json"
        return
    }
    $captureProcess.WaitForExit()
    if ($captureProcess.ExitCode -ne 0) { throw 'Tracy capture failed; no flamegraph may be claimed' }
    & $export -u -s ';' "$outputPath/capture.tracy" > "$outputPath/cpu-zones.csv"
    if ($LASTEXITCODE -ne 0) { throw 'CPU zone export failed' }
    & $export -g -s ';' "$outputPath/capture.tracy" > "$outputPath/gpu-zones.csv"
    if ($LASTEXITCODE -ne 0) { throw 'GPU zone export failed' }
    & $export -u -f benchmark_sample -s ';' "$outputPath/capture.tracy" > "$outputPath/markers.csv"
    if ($LASTEXITCODE -ne 0) { throw 'Sample marker export failed' }
    $analyzedView = if ($Combat -or $Generators) { 15 } else { $View }
    & python tools/profile_rendering.py "$outputPath/cpu-zones.csv" "$outputPath/analysis" --view $analyzedView --markers "$outputPath/markers.csv" --gpu "$outputPath/gpu-zones.csv" --performance "$outputPath/performance.json"
    if ($LASTEXITCODE -ne 0) { throw 'Flamegraph nesting/conservation validation failed' }
    if ($Memory -and $AllocationExporter) {
        $analysis = Get-Content -LiteralPath "$outputPath/analysis/flamegraph.json" -Raw | ConvertFrom-Json
        & $AllocationExporter -a $analysis.window_ns[0] -b $analysis.window_ns[1] "$outputPath/capture.tracy" > "$outputPath/allocations.csv"
        if ($LASTEXITCODE -ne 0) { throw 'Bounded native allocation export failed' }
        & python tools/profile_rendering.py "$outputPath/cpu-zones.csv" "$outputPath/analysis" --allocations "$outputPath/allocations.csv" --append-allocations
        if ($LASTEXITCODE -ne 0) { throw 'Bounded allocation validation failed' }
    } elseif ($Memory) {
        Write-Warning 'Allocation events remain in capture.tracy; supply -AllocationExporter for bounded heap analysis. No RAM improvement is established by the capture alone.'
    }
    Write-Output "$outputPath/analysis/flamegraph.svg"
} finally {
    if ($memoryObserver) {
        $memoryObserver | Stop-Job -ErrorAction SilentlyContinue
        $memoryObserver | Remove-Job -Force -ErrorAction SilentlyContinue
    }
    if ($captureProcess -and -not $captureProcess.HasExited) {
        Stop-Process -Id $captureProcess.Id
    }
    foreach ($key in $savedEnvironment.Keys) {
        [Environment]::SetEnvironmentVariable($key, $savedEnvironment[$key], 'Process')
    }
    Pop-Location
}
