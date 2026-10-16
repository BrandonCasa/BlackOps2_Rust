param(
    [ValidateSet('dev', 'staging', 'release')]
    [string] $Profile = 'dev',
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]] $GameArguments
)

$ErrorActionPreference = 'Stop'
$profileDirectory = if ($Profile -eq 'dev') { 'debug' } else { $Profile }
$executable = Join-Path $PSScriptRoot "../target/$profileDirectory/origins.exe"
if (-not (Test-Path -LiteralPath $executable)) {
    throw "Build Origins first: cargo build --profile $Profile --locked"
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
