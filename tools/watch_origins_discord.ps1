[CmdletBinding()]
param(
    [ValidateSet('Start', 'Run', 'Stop', 'Status')]
    [string]$Action = 'Start',
    [string]$StateDirectory = (Join-Path $env:LOCALAPPDATA 'OriginsDiscord'),
    [int]$Port = 19222
)

$ErrorActionPreference = 'Stop'
$statePath = [System.IO.Path]::GetFullPath($StateDirectory)
$statusPath = Join-Path $statePath 'status.json'
$stopPath = Join-Path $statePath 'stop.request'
$configPath = Join-Path $statePath 'config.json'
$workerPath = Join-Path $PSScriptRoot 'origins_discord.py'
$pidPath = Join-Path $statePath 'watcher.pid'
New-Item -ItemType Directory -Path $statePath -Force | Out-Null

if ($Action -eq 'Status') {
    if (Test-Path -LiteralPath $statusPath) { Get-Content -LiteralPath $statusPath }
    else { Write-Output 'Watcher has not started.' }
    exit
}

if ($Action -eq 'Stop') {
    Set-Content -LiteralPath $stopPath -Value 'stop' -Encoding utf8
    Write-Output 'Normal stop requested. Any upload already in progress will finish first.'
    exit
}

if (-not (Test-Path -LiteralPath $configPath)) {
    throw "Missing destination configuration: $configPath"
}

if ($Action -eq 'Start') {
    if (Test-Path -LiteralPath $pidPath) {
        $existingPid = [int](Get-Content -LiteralPath $pidPath -Raw).Trim()
        $existingProcess = Get-CimInstance Win32_Process -Filter "ProcessId = $existingPid"
        if ($existingProcess -and $existingProcess.CommandLine -like '*watch_origins_discord.ps1*') {
            Write-Output "Watcher is already running (PID $existingPid)."
            exit
        }
    }
    Remove-Item -LiteralPath $stopPath -Force -ErrorAction SilentlyContinue
    $powerShellPath = (Get-Process -Id $PID).Path
    $process = Start-Process -FilePath $powerShellPath -ArgumentList @(
        '-NoProfile', '-File', ('"{0}"' -f $PSCommandPath),
        '-Action', 'Run', '-StateDirectory', ('"{0}"' -f $statePath), '-Port', $Port
    ) -WindowStyle Hidden -PassThru
    Set-Content -LiteralPath $pidPath -Value $process.Id -Encoding ascii
    Write-Output "Watcher started in the background (PID $($process.Id))."
    exit
}

$runMutex = [System.Threading.Mutex]::new($false, "Local\OriginsDiscord-$Port")
if (-not $runMutex.WaitOne(0)) {
    Write-Output 'Watcher is already running.'
    exit
}
Set-Content -LiteralPath $pidPath -Value $PID -Encoding ascii

$uvPath = (Get-Command uv -ErrorAction Stop).Source
try {
    $endpoint = Invoke-RestMethod -Uri "http://127.0.0.1:$Port/json/version" -TimeoutSec 3
    if ($endpoint.'User-Agent' -notmatch 'discord/') { throw 'The control port belongs to another application.' }
} catch {
    $discordProcesses = Get-Process -Name Discord -ErrorAction SilentlyContinue
    if ($discordProcesses) {
        throw 'Discord is running without the background control connection. Quit Discord normally, then restart this watcher.'
    }
    $discordRoot = Join-Path $env:LOCALAPPDATA 'Discord'
    $discordExe = Get-ChildItem -LiteralPath $discordRoot -Directory -Filter 'app-*' |
        Sort-Object LastWriteTime -Descending |
        ForEach-Object { Join-Path $_.FullName 'Discord.exe' } |
        Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1
    if (-not $discordExe) { throw 'Discord desktop installation was not found.' }
    Start-Process -FilePath $discordExe -ArgumentList "--remote-debugging-port=$Port", '--remote-debugging-address=127.0.0.1' -WindowStyle Hidden
    $deadline = [DateTime]::UtcNow.AddSeconds(60)
    do {
        if (Test-Path -LiteralPath $stopPath) { exit }
        Start-Sleep -Seconds 1
        try {
            $endpoint = Invoke-RestMethod -Uri "http://127.0.0.1:$Port/json/version" -TimeoutSec 2
            if ($endpoint.'User-Agent' -match 'discord/') { break }
        } catch { }
    } while ([DateTime]::UtcNow -lt $deadline)
    if (-not $endpoint) { throw 'Discord background connection did not start.' }
}

& $uvPath run --python 3.12 --with windows-capture==2.0.1 --with pillow==12.3.0 --with websocket-client==1.9.0 python -B $workerPath --port $Port --state-dir $statePath --config $configPath
exit $LASTEXITCODE
