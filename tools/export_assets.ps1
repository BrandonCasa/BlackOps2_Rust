[CmdletBinding()]
param([switch]$SkipOat, [switch]$GameplayOnly)
$ErrorActionPreference = 'Stop'
$checkout = Split-Path $PSScriptRoot -Parent
$steam = Join-Path ${env:ProgramFiles(x86)} 'Steam/steamapps/common/Call of Duty Black Ops II'
$revision = '088dac3bdee0e6e653871f877ad8ee4326d314a4'
$tools = Join-Path $checkout 'target/tools'
$extracted = Join-Path $checkout 'target/extracted'
$assets = Join-Path $checkout 'target/assets'
$reports = Join-Path $checkout 'target/reports'
$zone = Join-Path $steam 'zone/all'
foreach ($directory in @($tools, $extracted, $assets, $reports)) {
    New-Item -ItemType Directory -Path $directory -Force | Out-Null
}
if (!(Test-Path -LiteralPath (Join-Path $zone 'zm_tomb.ff'))) { throw 'Authorized Steam Origins fastfile is missing.' }
$authoredGame = Join-Path $checkout 'gameplay/origins/game.json'
if (!(Test-Path -LiteralPath $authoredGame)) { throw 'Tracked game-owned gameplay/origins/game.json is missing.' }
Copy-Item -LiteralPath $authoredGame -Destination (Join-Path $assets 'game.json') -Force
if ($GameplayOnly) { return }


# First-party mpv CI binary: stock synchronized playback, no generated cinematic frames.
$mpv = Join-Path $tools 'mpv/mpv.exe'
if (!(Test-Path -LiteralPath $mpv)) {
    $archive = Join-Path $tools 'mpv-v0.41.0-dev-g7c4f662c0-38598545310-x86_64-w64-mingw32-full.zip'
    Invoke-WebRequest 'https://github.com/mpv-player/mpv/releases/download/git-release/mpv-v0.41.0-dev-g7c4f662c0-38598545310-x86_64-w64-mingw32-full.zip' -OutFile $archive
    if ((Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash -ne '0C310AA06B77BBF603CA99B024F0CBD5D85BBCEB4C106235373D844DBDA941EC') { throw 'Pinned mpv archive hash mismatch.' }
    Expand-Archive -LiteralPath $archive -DestinationPath (Join-Path $tools 'mpv') -Force
}
# Unmodified, pinned upstream tools; all downloads and private dependencies stay in target.
$oat = Join-Path $tools 'oat'
$unlinker = Join-Path $oat 'Unlinker.exe'
if (!(Test-Path -LiteralPath $unlinker)) {
    $release = Invoke-RestMethod 'https://api.github.com/repos/Laupetin/OpenAssetTools/releases/tags/v0.33.0'
    $download = @($release.assets | Where-Object { $_.name -match '(?i)windows.*\.zip$' })
    if ($download.Count -ne 1) { throw 'Expected one Windows ZIP in OAT v0.33.0 release.' }
    $archive = Join-Path $tools 'oat-v0.33.0.zip'
    Invoke-WebRequest $download[0].browser_download_url -OutFile $archive
    Expand-Archive -LiteralPath $archive -DestinationPath $oat
}
$version = (& $unlinker --version 2>&1 | Out-String)
if ($LASTEXITCODE -ne 0 -or $version -notmatch '0\.33\.0') { throw "Expected OAT v0.33.0: $version" }
$external = Join-Path $tools "open-fastfile-$revision"
if (!(Test-Path -LiteralPath (Join-Path $external 'scripts/cod/__main__.py'))) {
    $archive = Join-Path $tools "open-fastfile-$revision.zip"
    Invoke-WebRequest "https://github.com/eithan1231/open-fastfile/archive/$revision.zip" -OutFile $archive
    Expand-Archive -LiteralPath $archive -DestinationPath $tools
}
$python = Join-Path $external '.venv/Scripts/python.exe'
if (!(Test-Path -LiteralPath $python)) {
    & python -m venv (Join-Path $external '.venv')
    if ($LASTEXITCODE -ne 0) { throw 'External exporter Python environment creation failed.' }
    & $python -m pip --cache-dir (Join-Path $tools 'pip-cache') install -r (Join-Path $external 'scripts/cod/requirements.txt')
    if ($LASTEXITCODE -ne 0) { throw 'External exporter dependency installation failed.' }
}

if (!$SkipOat) {
# Official Tom Crowley iPak Exporter supplies the images OAT cannot find.
$ipakTool = Join-Path $tools 'ipak-exporter/Tom_BO2_iPak/Tom_BO2_iPak.exe'
if (!(Test-Path -LiteralPath $ipakTool)) {
    $archive = Join-Path $tools 'ipak-exporter.rar'
    Invoke-WebRequest 'https://tom-crowley.co.uk/downloads/15' -OutFile $archive
    & 'C:/Program Files/7-Zip/7z.exe' x $archive "-o$(Join-Path $tools 'ipak-exporter')" -y
    if ($LASTEXITCODE -ne 0) { throw 'Official iPak Exporter archive extraction failed.' }
}
$texconv = Join-Path $tools 'texconv/texconv.exe'
if (!(Test-Path -LiteralPath $texconv)) {
    New-Item -ItemType Directory -Path (Split-Path $texconv -Parent) -Force | Out-Null
    Invoke-WebRequest 'https://github.com/microsoft/DirectXTex/releases/download/may2026/texconv.exe' -OutFile $texconv
}
$images = Join-Path $extracted 'images'
New-Item -ItemType Directory -Path $images -Force | Out-Null
Push-Location (Split-Path $ipakTool -Parent)
try {
    foreach ($pack in @('base', 'zm', 'dlczm4', 'patch_zm')) {
        $rawImages = Join-Path $extracted "ipak-$pack"
        $inputFile = Join-Path $tools 'ipak-exit-input.txt'
        [IO.File]::WriteAllText($inputFile, "`r`n")
        $export = Start-Process -FilePath $ipakTool -ArgumentList @('-f', "`"$(Join-Path $zone "$pack.ipak")`"", '-o', "`"$rawImages`"", '-n', '-ra', '-d') -WindowStyle Hidden -PassThru -RedirectStandardInput $inputFile -RedirectStandardOutput (Join-Path $reports "ipak-$pack.log") -RedirectStandardError (Join-Path $reports "ipak-$pack-errors.log")
        $completed = $false
        try {
            $deadline = [DateTime]::UtcNow.AddMinutes(5)
            do {
                $export.Refresh()
                $summary = Select-String -LiteralPath (Join-Path $reports "ipak-$pack.log") -Pattern '^Export Time: ' -ErrorAction SilentlyContinue
                if ($summary) { $completed = $true; break }
                if ($export.HasExited) { break }
                Start-Sleep -Milliseconds 200
            } while ([DateTime]::UtcNow -lt $deadline)
        } finally {
            # This process was created above; its legacy exit prompt ignores redirected stdin.
            $export.Refresh()
            if (!$export.HasExited) { Stop-Process -Id $export.Id; $export.WaitForExit() }
        }
        if (!$completed) { throw "External iPak export did not report completion: $pack" }
        $runtimeImages = Join-Path $assets 'images'
        New-Item -ItemType Directory -Path $runtimeImages -Force | Out-Null
        Copy-Item "$rawImages/*.dds" -Destination $runtimeImages -Force
        & $texconv -ft dds -f R8G8B8A8_UNORM -y -o $runtimeImages "$rawImages/*.tga" *> (Join-Path $reports "normals-$pack.log")
        if ($LASTEXITCODE -ne 0) { throw "External normal DDS conversion failed: $pack" }
        # Exported source channels remain stored intact; built-in runtime materials apply only supported mappings.
        & $texconv -ft png -f R8G8B8A8_UNORM -y -o $images "$rawImages/*.dds" "$rawImages/*.tga" *> (Join-Path $reports "images-$pack.log")
        if ($LASTEXITCODE -ne 0) { throw "External PNG conversion failed: $pack" }
    }
} finally { Pop-Location }
Copy-Item -LiteralPath $images -Destination $assets -Recurse -Force

}

$oatFailed = $false
if (!$SkipOat) {
    $oatOutput = Join-Path $extracted 'oat/?zone?'
    $oatLog = Join-Path $reports 'oat-assets.log'
    & $unlinker --no-color --load (Join-Path $zone 'code_post_gfx_zm.ff') --load (Join-Path $steam 'zone/english/en_code_post_gfx_zm.ff') --model-format GLB --image-format DDS --output-folder $oatOutput --search-path "$zone;$(Join-Path $steam 'sound');$(Join-Path $steam 'zone/english')" (Join-Path $zone 'code_post_gfx_zm.ff') (Join-Path $steam 'zone/english/en_code_post_gfx_zm.ff') (Join-Path $zone 'common_zm.ff') (Join-Path $zone 'patch_zm.ff') (Join-Path $zone 'zm_tomb.ff') (Join-Path $zone 'zm_tomb_patch.ff') (Join-Path $steam 'zone/english/en_zm_tomb.ff') *> $oatLog
    $oatExit = $LASTEXITCODE
    $summaries = @(Select-String -LiteralPath $oatLog -Pattern 'Finished with.*?(\d+) errors')
    $reportedErrors = @($summaries | Where-Object { [int]$_.Matches[0].Groups[1].Value -gt 0 })
    $oatFailed = $oatExit -ne 0 -or $summaries.Count -eq 0 -or $reportedErrors.Count -gt 0 -or [bool](Select-String -LiteralPath $oatLog -Pattern '^ERROR:')
    foreach ($name in @('code_post_gfx_zm', 'en_code_post_gfx_zm', 'common_zm', 'patch_zm', 'zm_tomb', 'zm_tomb_patch')) {
        $sourceSound = Join-Path $extracted "oat/$name/sound"
        if (Test-Path -LiteralPath $sourceSound) {
            Copy-Item -LiteralPath $sourceSound -Destination $assets -Recurse -Force
        }
    }
    foreach ($name in @('code_post_gfx_zm', 'en_code_post_gfx_zm', 'common_zm', 'patch_zm', 'zm_tomb', 'zm_tomb_patch')) {
        $sourceImages = Join-Path $extracted "oat/$name/images"
        if (Test-Path -LiteralPath $sourceImages) { Get-ChildItem $sourceImages -File -Filter '*.dds' | Copy-Item -Destination (Join-Path $assets 'images') -Force }
    }
    $english = Join-Path $extracted 'oat/en_zm_tomb/english' 
    if (Test-Path -LiteralPath $english) { Copy-Item -LiteralPath $english -Destination $assets -Recurse -Force }
    & $unlinker --no-color --include-assets snddriverglobals --output-folder (Join-Path $extracted 'oat-driver/?zone?') (Join-Path $zone 'code_post_gfx_zm.ff') *> (Join-Path $reports 'oat-driver.log')
    if ($LASTEXITCODE -ne 0) { throw 'OAT sound-driver globals export failed.' }
    $globals = Join-Path $extracted 'oat-driver/code_post_gfx_zm/soundbank/globals'
    $audio = Join-Path $assets 'audio'
    New-Item -ItemType Directory -Path $audio -Force | Out-Null
    Copy-Item -LiteralPath $globals -Destination $audio -Recurse -Force
    & $python (Join-Path $checkout 'tools/export_audio.py') --tool $external --game $steam --out $assets *> (Join-Path $reports 'external-audio-aliases.log')
    if ($LASTEXITCODE -ne 0) { throw 'External native audio alias export failed.' }
    foreach ($movie in @('zm_tomb_load', 'zm_tomb_ee')) {
        Copy-Item -LiteralPath (Join-Path $steam "video/$movie.webm") -Destination (Join-Path $assets "cinematics/$movie.webm") -Force
    }
    if ($oatFailed) { Write-Warning "OAT extraction is incomplete; inspect $oatLog. Collision export will still run." }
}

$work = Join-Path $extracted 'origins-collision-work'
$output = Join-Path $extracted 'origins-collision'
$collisionLog = Join-Path $reports 'external-collision.log'
Push-Location $external
try {
    & $python -m scripts.cod export zm_tomb --game bo2 --game-dir "bo2=$steam" --zone $zone --search $zone --out $output --work $work --keep-work --collision-only *> $collisionLog
    $collisionExit = $LASTEXITCODE
    # Upstream Windows path handling looks in gw for this existing entity dump.
    # Copy the file unchanged and rerun the exact same unmodified external command.
    $entities = Join-Path $work 'ents/maps/mp/zm_tomb.d3dbsp.ents'
    if (Test-Path -LiteralPath $entities) {
        $destination = Join-Path $work 'gw/maps/mp/zm_tomb.d3dbsp.ents'
        New-Item -ItemType Directory -Path (Split-Path $destination -Parent) -Force | Out-Null
        Copy-Item -LiteralPath $entities -Destination $destination -Force
        & $python -m scripts.cod export zm_tomb --game bo2 --game-dir "bo2=$steam" --zone $zone --search $zone --out $output --work $work --keep-work --collision-only *>> $collisionLog
        $collisionExit = $LASTEXITCODE
    }
} finally { Pop-Location }
$collision = Join-Path $output 'assets/collision.glb'
$clipmap = Join-Path $work 'clip/maps/mp/zm_tomb.d3dbsp.clipmap.json'
if ($collisionExit -ne 0 -or !(Test-Path -LiteralPath $collision) -or !(Test-Path -LiteralPath $clipmap)) {
    throw "External collision export failed or required outputs are absent: $collisionLog"
}
Copy-Item -LiteralPath $collision -Destination (Join-Path $assets 'collision.glb') -Force
# Official glTF-Transform converts only the unsupported scalar-byte metadata encoding.
# Source values, positions, triangles and winding remain intact.
$gltfTools = Join-Path $tools 'gltf-transform'
$gltfCli = Join-Path $gltfTools 'node_modules/.bin/gltf-transform.cmd'
if (!(Test-Path -LiteralPath $gltfCli)) {
    & npm.cmd install --prefix $gltfTools --cache (Join-Path $tools 'npm-cache') --no-audit --no-fund '@gltf-transform/cli@4.5.0' *> (Join-Path $reports 'gltf-transform-install.log')
    if ($LASTEXITCODE -ne 0) { throw 'glTF-Transform installation failed.' }
}
& $gltfCli dequantize $collision (Join-Path $assets 'collision-bevy.glb') --pattern '_SURFACE' *> (Join-Path $reports 'collision-bevy.log')
if ($LASTEXITCODE -ne 0) { throw 'Collision surface encoding conversion failed.' }

Copy-Item -LiteralPath $clipmap -Destination (Join-Path $assets 'clipmap.json') -Force
Write-Host 'Staged external collision.glb and raw clipmap.json only. Full collision semantics remain subject to gameplay verification.'
if (!$SkipOat) {
    Push-Location $external
    try {
        & $python (Join-Path $checkout 'tools/export_world.py') $checkout $external $zone *> (Join-Path $reports 'external-world.log')
        if ($LASTEXITCODE -ne 0) { throw 'External world export failed: inspect target/reports/external-world.log.' }
    } finally { Pop-Location }
    & $python (Join-Path $checkout 'tools/export_world_inputs.py') $checkout $external (Join-Path $zone 'zm_tomb.ff') *> (Join-Path $reports 'external-world-inputs.log')
    if ($LASTEXITCODE -ne 0) { throw 'Original world shader input export failed: inspect target/reports/external-world-inputs.log.' }
    # Preserve source texels and mip counts while replacing unsupported BGRX DDS.
    & $python (Join-Path $checkout 'tools/normalize_dds.py') (Join-Path $assets 'images') $texconv (Join-Path $reports 'opaque-dds-normalization.json')
    if ($LASTEXITCODE -ne 0) { throw 'Original opaque DDS normalization failed.' }
    foreach ($name in @('code_post_gfx_zm', 'en_code_post_gfx_zm', 'common_zm', 'patch_zm', 'zm_tomb', 'zm_tomb_patch')) {
        $models = Join-Path $extracted "oat/$name/model_export"
        if (Test-Path -LiteralPath $models) { Copy-Item -LiteralPath $models -Destination $assets -Recurse -Force }
    }
    $animationArguments = @((Join-Path $checkout 'tools/export_animations.py'), '--oat', (Join-Path $extracted 'oat/zm_tomb'), '--overlay', (Join-Path $extracted 'oat/patch_zm'), '--tool', $external, '--out', $assets, '--report', (Join-Path $reports 'full-animation-coverage.json'))
    $definitionPath = Join-Path $assets 'game.json'
    if (Test-Path -LiteralPath $definitionPath) { $animationArguments += @('--game-definition', $definitionPath) }
    & $python @animationArguments *> (Join-Path $reports 'external-animations.log')
    if ($LASTEXITCODE -ne 0) { throw 'External animation export failed: inspect target/reports/external-animations.log and full-animation-coverage.json.' }
    # OAT's nested mlv scenes retain ../images URIs; expose the same ordinary files there.
    $variantImages = Join-Path $assets 'model_export/images'
    New-Item -ItemType Directory -Path $variantImages -Force | Out-Null
    Get-ChildItem (Join-Path $assets 'images') -File | ForEach-Object {
        $destination = Join-Path $variantImages $_.Name
        if (!(Test-Path -LiteralPath $destination)) { New-Item -ItemType HardLink -Path $destination -Target $_.FullName | Out-Null }
    }
    # OAT model copies can overwrite existing aliases after native RGBA staging.
    & $python (Join-Path $checkout 'tools/native_images.py') --assets $assets --report (Join-Path $reports 'native-rgba-source-restage.json') --sync-aliases
    if ($LASTEXITCODE -ne 0) { throw 'Original source image alias synchronization failed.' }
}

# Native materials own texture loading; glTF retains geometry, skins and animation.
& $python (Join-Path $checkout 'tools/strip_native_rig_materials.py') --assets $assets --report (Join-Path $reports 'native-rig-material-removal.json')
if ($LASTEXITCODE -ne 0) { throw 'Native rig material removal failed.' }

# Normalize reserved URI characters without rewriting model binary chunks.
& $python (Join-Path $checkout 'tools/normalize_gltf_uris.py') --assets $assets *> (Join-Path $reports 'gltf-uri-normalization.json')
if ($LASTEXITCODE -ne 0) { throw 'glTF source image URI normalization failed; inspect gltf-uri-normalization.json.' }

if ($oatFailed) { throw 'OAT reported extraction errors. This export is incomplete; no complete asset set was produced.' }
# Recontainer source GPU blocks and raw texels without invoking an encoder.
$ktxSdk = Join-Path $tools 'ktx/sdk'
if (!(Test-Path -LiteralPath (Join-Path $ktxSdk 'bin/ktx.dll'))) {
    $archive = Join-Path $tools 'ktx/installer.exe'
    New-Item -ItemType Directory -Path (Split-Path $archive -Parent) -Force | Out-Null
    Invoke-WebRequest 'https://github.com/KhronosGroup/KTX-Software/releases/download/v4.4.2/KTX-Software-4.4.2-Windows-x64.exe' -OutFile $archive
    if ((Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash -ne '1F323B0FEC19794F5E6C0425A61D4B1DA396872A10BE862D105F4F4B2D2957FE') { throw 'Pinned KTX SDK archive hash mismatch.' }
    & 'C:/Program Files/7-Zip/7z.exe' x $archive "-o$ktxSdk" -y
    if ($LASTEXITCODE -ne 0) { throw 'KTX SDK extraction failed.' }
}
& $python (Join-Path $checkout 'tools/repack_ktx2.py') --assets $assets --library (Join-Path $ktxSdk 'bin/ktx.dll') --report (Join-Path $reports 'ktx2-repack.json')
if ($LASTEXITCODE -ne 0) { throw 'Lossless KTX2 repack failed.' }
& $python (Join-Path $checkout 'tools/normalize_gltf_uris.py') --assets $assets --ktx2 *> (Join-Path $reports 'gltf-ktx2.json')
if ($LASTEXITCODE -ne 0) { throw 'glTF KTX2 URI migration failed.' }
if (!(Test-Path -LiteralPath (Join-Path $assets 'game.json'))) {
    throw 'Existing game-owned target/assets/game.json is required. This script does not generate gameplay data, complete models/materials/animations, or launch the game.'
}
Write-Host "Existing game.json found. Full world/actor/weapon/image/audio dependency coverage is still required before launching with --asset-root $assets."
