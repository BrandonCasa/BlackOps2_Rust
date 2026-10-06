$ErrorActionPreference = 'Stop'
$repository = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
Push-Location $repository
try {
    $cargoCommand = Get-Command cargo -ErrorAction SilentlyContinue
    $cargoExecutable = if ($cargoCommand) { $cargoCommand.Source } else { Join-Path $env:USERPROFILE '.cargo/bin/cargo.exe' }
    & $cargoExecutable metadata --locked --no-deps --format-version 1 | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'Cargo metadata failed' }
    & $cargoExecutable build --workspace --release --locked
    if ($LASTEXITCODE -ne 0) { throw 'Release build failed' }
    & $cargoExecutable fmt --all -- --check
    if ($LASTEXITCODE -ne 0) { throw 'Formatting check failed' }
    $release = Join-Path $repository 'release'
    & uv run --python '>=3.11' --with 'numpy>=2,<3' --with 'pillow>=12,<13' python -c 'from pathlib import Path; from tools.import_origins import import_pack; result = import_pack(Path("assets/origins"), Path("release/assets/origins"), None); print({k: v for k, v in result.items() if k != "sha256"})'
    if ($LASTEXITCODE -ne 0) { throw 'Runtime asset validation/copy failed; inspect the error above (LFS pointers require git lfs pull)' }
    Copy-Item -LiteralPath (Join-Path $repository 'target/release/origins.exe') -Destination (Join-Path $release 'origins.exe')
    # Retire the old package's machine-specific configuration, if present.
    $oldConfig = [IO.Path]::GetFullPath((Join-Path $release 'config/local_assets.toml'))
    if (-not $oldConfig.StartsWith($release + [IO.Path]::DirectorySeparatorChar)) { throw 'Invalid release config path' }
    if (Test-Path -LiteralPath $oldConfig) { Remove-Item -LiteralPath $oldConfig }
    Write-Output (Join-Path $release 'origins.exe')
} finally { Pop-Location }
