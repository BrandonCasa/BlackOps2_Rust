param(
    [Parameter(Mandatory = $true)][string]$Output,
    [Parameter(Mandatory = $true)][string]$TracyDirectory,
    [int]$View = 8,
    [switch]$AllViews,
    [switch]$Combat
)
$ErrorActionPreference = 'Stop'
$repository = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
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
foreach ($key in @('NO_COLOR', 'RUST_LOG', 'TRACY_NO_SYS_TRACE', 'TRACY_SYMBOL_OFFLINE_RESOLVE')) {
    $savedEnvironment[$key] = [Environment]::GetEnvironmentVariable($key, 'Process')
}
Push-Location $repository
try {
    & cargo build --workspace --release --locked --features profile-tracy
    if ($LASTEXITCODE -ne 0) { throw 'Profiling release build failed' }
    New-Item -ItemType Directory -Path $outputPath | Out-Null
    $env:NO_COLOR = '1'
    $env:RUST_LOG = 'info,wgpu=error,naga=warn'
    # Named ECS/GPU zones remain enabled; ETW sampling and online symbols are optional.
    $env:TRACY_NO_SYS_TRACE = '1'
    $env:TRACY_SYMBOL_OFFLINE_RESOLVE = '1'
    $captureProcess = Start-Process $capture -ArgumentList @('-o', "`"$outputPath/capture.tracy`"", '-a', '127.0.0.1', '-s', '350') -WindowStyle Hidden -RedirectStandardOutput "$outputPath/capture.log" -RedirectStandardError "$outputPath/capture-errors.log" -PassThru
    $gameArguments = if ($Combat) { @('--benchmark-combat', $outputPath) } elseif ($AllViews) { @('--benchmark', $outputPath) } else { @('--benchmark-view', $outputPath, $View) }
    & (Join-Path $repository 'target/release/origins.exe') @gameArguments 2> "$outputPath/game.log"
    if ($LASTEXITCODE -ne 0) { throw "Actual game benchmark failed; inspect $outputPath/game.log" }
    $captureProcess.WaitForExit()
    if ($captureProcess.ExitCode -ne 0) { throw 'Tracy capture failed; no flamegraph may be claimed' }
    & $export -u -s ';' "$outputPath/capture.tracy" > "$outputPath/cpu-zones.csv"
    if ($LASTEXITCODE -ne 0) { throw 'CPU zone export failed' }
    & $export -g -s ';' "$outputPath/capture.tracy" > "$outputPath/gpu-zones.csv"
    if ($LASTEXITCODE -ne 0) { throw 'GPU zone export failed' }
    & $export -u -f benchmark_sample -s ';' "$outputPath/capture.tracy" > "$outputPath/markers.csv"
    if ($LASTEXITCODE -ne 0) { throw 'Sample marker export failed' }
    $analyzedView = if ($Combat) { 15 } else { $View }
    & python tools/profile_rendering.py "$outputPath/cpu-zones.csv" "$outputPath/analysis" --view $analyzedView --markers "$outputPath/markers.csv"
    if ($LASTEXITCODE -ne 0) { throw 'Flamegraph nesting/conservation validation failed' }
    Write-Output "$outputPath/analysis/flamegraph.svg"
} finally {
    if ($captureProcess -and -not $captureProcess.HasExited) {
        Stop-Process -Id $captureProcess.Id
    }
    foreach ($key in $savedEnvironment.Keys) {
        [Environment]::SetEnvironmentVariable($key, $savedEnvironment[$key], 'Process')
    }
    Pop-Location
}
