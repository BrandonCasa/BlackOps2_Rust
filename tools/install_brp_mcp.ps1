param(
    [string]$BuildDirectory,
    [ValidateRange(1, 64)]
    [int]$Jobs = 6
)

$ErrorActionPreference = 'Stop'
$installArguments = @('install', 'bevy_brp_mcp', '--version', '0.22.10', '--locked', '--jobs', $Jobs)
if ($BuildDirectory) {
    $installArguments += @('--target-dir', $BuildDirectory)
}
& cargo @installArguments
if ($LASTEXITCODE -ne 0) {
    throw "bevy_brp_mcp installation failed with exit code $LASTEXITCODE"
}
