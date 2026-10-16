param(
    [switch] $Release,
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]] $GameArguments
)

$ErrorActionPreference = 'Stop'
$profileDirectory = if ($Release) { 'release' } else { 'debug' }
$executable = Join-Path $PSScriptRoot "../target/$profileDirectory/origins.exe"
if (-not (Test-Path -LiteralPath $executable)) {
    throw "Build Origins first: cargo build$(if ($Release) { ' --release' })"
}

$start = [System.Diagnostics.ProcessStartInfo]::new($executable)
$start.UseShellExecute = $false
$start.CreateNoWindow = $true
# Use each layer's declared opt-out, scoped to this game process.
$start.Environment['DISABLE_VULKAN_OW_OBS_CAPTURE'] = '1'
$start.Environment['DISABLE_VK_LAYER_reshade_1'] = '1'
foreach ($argument in $GameArguments) {
    $start.ArgumentList.Add($argument)
}
$game = [System.Diagnostics.Process]::Start($start)
$game.WaitForExit()
exit $game.ExitCode
