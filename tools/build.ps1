param(
    [ValidateSet('Debug', 'Release')][string]$Profile = 'Debug',
    [ValidateSet('Build', 'Check')][string]$CompilerGate = 'Build',
    [uint32]$CompilerSeconds = 0,
    [switch]$Ddgi
)

$ErrorActionPreference = 'Stop'
$repository = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$savedRustc = $env:RUSTC
$savedRustdoc = $env:RUSTDOC
Push-Location $repository
try {
    $cargoCommand = Get-Command cargo -ErrorAction SilentlyContinue
    $cargoExecutable = if ($cargoCommand) { $cargoCommand.Source } else { Join-Path $env:USERPROFILE '.cargo/bin/cargo.exe' }
    # Launch the actual Cargo process, not the rustup shim. This keeps its child
    # compiler tree attached to the PID we own when a bounded gate must stop.
    $rustupCommand = Get-Command rustup -ErrorAction SilentlyContinue
    if ($rustupCommand) {
        $resolvedCargo = & $rustupCommand.Source which cargo
        if ($LASTEXITCODE -eq 0 -and (Test-Path -LiteralPath $resolvedCargo)) {
            $cargoExecutable = $resolvedCargo
            # Direct Cargo bypasses rustup's RUSTUP_TOOLCHAIN propagation.
            # Resolve compiler tools here, before Cargo enters dependency dirs
            # that may contain their own unrelated rust-toolchain.toml.
            $env:RUSTC = & $rustupCommand.Source which rustc
            if ($LASTEXITCODE -ne 0) { throw 'Could not resolve workspace rustc' }
            $env:RUSTDOC = & $rustupCommand.Source which rustdoc
            if ($LASTEXITCODE -ne 0) { throw 'Could not resolve workspace rustdoc' }
        }
    }
    $occupied = Get-Process -Name origins,cargo,rustc,capture,wpr,xperf,texconv -ErrorAction SilentlyContinue
    if ($occupied) { throw 'Build deferred while the shared game/capture/compiler slot is occupied' }
    function Invoke-Compiler([string[]]$compilerArguments) {
        $logRoot = Join-Path $repository 'target'
        New-Item -ItemType Directory -Path $logRoot -Force | Out-Null
        $stdout = Join-Path $logRoot 'compiler-current.stdout.log'
        $stderr = Join-Path $logRoot 'compiler-current.stderr.log'
        $compiler = Start-Process -FilePath $cargoExecutable -ArgumentList $compilerArguments -WorkingDirectory $repository -RedirectStandardOutput $stdout -RedirectStandardError $stderr -WindowStyle Hidden -PassThru
        $timer = [Diagnostics.Stopwatch]::StartNew()
        $interrupted = $false
        while (-not $compiler.HasExited) {
            if (($CompilerSeconds -gt 0 -and $timer.Elapsed.TotalSeconds -ge $CompilerSeconds) -or
                (Get-Process -Name origins,capture,wpr,xperf,texconv -ErrorAction SilentlyContinue)) {
                $owned = Get-CimInstance Win32_Process -Filter "ProcessId=$($compiler.Id)"
                if ($owned) {
                    if ($owned.Name -ne 'cargo.exe' -or ($owned.ExecutablePath -and $owned.ExecutablePath -ne $cargoExecutable)) {
                        throw 'Owned compiler identity changed; refusing to stop another process'
                    }
                    & taskkill.exe /PID $compiler.Id /T /F | Out-Null
                    if ($LASTEXITCODE -ne 0) { throw 'Could not stop the owned compiler tree' }
                }
                $interrupted = $true
                break
            }
            Start-Sleep -Milliseconds 250
            $compiler.Refresh()
        }
        if (-not $compiler.WaitForExit(10000)) { throw 'Owned compiler failed to retire after stop' }
        Get-Content -LiteralPath $stdout
        Get-Content -LiteralPath $stderr
        if ($interrupted) { throw "Compiler gate stopped after $([math]::Round($timer.Elapsed.TotalSeconds, 1)) seconds; no compiler-pass claim" }
        if ($compiler.ExitCode -ne 0) { throw "Compiler gate failed with exit code $($compiler.ExitCode)" }
    }
    & $cargoExecutable metadata --locked --no-deps --format-version 1 | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'Cargo metadata failed' }
    if ($CompilerGate -eq 'Check') {
        $checkArguments = @('check', '--lib', '--offline', '--locked', '--jobs', '1')
        if ($Profile -eq 'Release') { $checkArguments += '--release' }
        else { $checkArguments += @('--features', 'dev-dynamic') }
        if ($Ddgi) { $checkArguments += @('--features', 'ddgi') }
        Invoke-Compiler -compilerArguments $checkArguments
        return
    }
    $buildArguments = @('build', '--workspace', '--locked')
    if ($Profile -eq 'Release') { $buildArguments += '--release' }
    else { $buildArguments += @('--features', 'dev-dynamic') }
    if ($Ddgi) { $buildArguments += @('--features', 'ddgi') }
    Invoke-Compiler -compilerArguments $buildArguments
    & $cargoExecutable fmt --all -- --check
    if ($LASTEXITCODE -ne 0) { throw 'Formatting check failed' }
    if ($Profile -eq 'Debug') {
        Write-Output (Join-Path $repository 'target/debug/origins.exe')
        return
    }
    $release = Join-Path $repository 'release'
    'from pathlib import Path; from tools.import_origins import import_pack; result = import_pack(Path("assets/origins"), Path("release/assets/origins"), None); print({k: v for k, v in result.items() if k != "sha256"})' | & uv run --python '>=3.11' --with 'numpy>=2,<3' --with 'pillow>=12,<13' --with 'soundfile>=0.13,<0.14' python -
    if ($LASTEXITCODE -ne 0) { throw 'Runtime asset validation/copy failed; inspect the error above (LFS pointers require git lfs pull)' }
    Copy-Item -LiteralPath (Join-Path $repository 'target/release/origins.exe') -Destination (Join-Path $release 'origins.exe')
    # Retire the old package's machine-specific configuration, if present.
    $oldConfig = [IO.Path]::GetFullPath((Join-Path $release 'config/local_assets.toml'))
    if (-not $oldConfig.StartsWith($release + [IO.Path]::DirectorySeparatorChar)) { throw 'Invalid release config path' }
    if (Test-Path -LiteralPath $oldConfig) { Remove-Item -LiteralPath $oldConfig }
    Write-Output (Join-Path $release 'origins.exe')
} finally {
    $env:RUSTC = $savedRustc
    $env:RUSTDOC = $savedRustdoc
    Pop-Location
}
