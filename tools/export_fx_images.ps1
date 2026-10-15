[CmdletBinding()]
param([Parameter(Mandatory=$true)][string]$Texconv)
$ErrorActionPreference = 'Stop'
$checkout = Split-Path $PSScriptRoot -Parent
$assets = Join-Path $checkout 'target/assets'
$reports = Join-Path $checkout 'target/reports'
$catalog = Get-Content -LiteralPath (Join-Path $checkout 'gameplay/origins/fx_catalog.json') -Raw | ConvertFrom-Json
# Official DirectXTex expands original DDS to lossless RGBA PNG. No pixels are synthesized.
$images = @($catalog.materials.PSObject.Properties | ForEach-Object { $_.Value.image } | Sort-Object -Unique)
$inputs = @($images | ForEach-Object {
    $source = Join-Path $assets ([IO.Path]::ChangeExtension($_, '.dds'))
    if (!(Test-Path -LiteralPath $source)) { throw "Required original FX image missing: $source" }
    $source
})
$list = Join-Path $reports 'fx-source-images.txt'
[IO.File]::WriteAllLines($list, $inputs)
& $Texconv -ft png -f R8G8B8A8_UNORM -y -o (Join-Path $assets 'images') @inputs *> (Join-Path $reports 'fx-texture-conversion.log')
if ($LASTEXITCODE -ne 0) { throw 'DirectXTex FX image conversion failed; inspect fx-texture-conversion.log.' }
foreach ($image in $images) {
    if (!(Test-Path -LiteralPath (Join-Path $assets $image))) { throw "Required converted FX image missing: $image" }
}
Write-Host "Converted $($images.Count) original FX textures to lossless RGBA PNG."
