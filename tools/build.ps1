$ErrorActionPreference = 'Stop'
$repository = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
Push-Location $repository
try {
    $cargoCommand = Get-Command cargo -ErrorAction SilentlyContinue
    $cargoExecutable = if ($cargoCommand) { $cargoCommand.Source } else { Join-Path $env:USERPROFILE '.cargo/bin/cargo.exe' }
    & $cargoExecutable build --release --locked
    if ($LASTEXITCODE -ne 0) { throw 'Release build failed' }
    New-Item -ItemType Directory -Force -Path (Join-Path $repository 'release/config') | Out-Null
    Copy-Item -LiteralPath (Join-Path $repository 'target/release/origins.exe') -Destination (Join-Path $repository 'release/origins.exe')
    Copy-Item -LiteralPath (Join-Path $repository 'config/local_assets.toml') -Destination (Join-Path $repository 'release/config/local_assets.toml')
    Write-Output (Join-Path $repository 'release/origins.exe')
} finally { Pop-Location }
