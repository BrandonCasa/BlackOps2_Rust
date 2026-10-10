param([int]$TargetProcess, [int]$LogicalCpus)
$ErrorActionPreference = 'Stop'
$timer = [Diagnostics.Stopwatch]::StartNew()
$previousTime = $timer.Elapsed.TotalSeconds
$previousCpu = (Get-Process -Id $TargetProcess).TotalProcessorTime.TotalSeconds
while ($true) {
    Start-Sleep -Milliseconds 1000
    $game = Get-Process -Id $TargetProcess -ErrorAction SilentlyContinue
    if (-not $game) { break }
    $now = $timer.Elapsed.TotalSeconds
    $elapsed = $now - $previousTime
    $cpuTime = $game.TotalProcessorTime.TotalSeconds
    $gpu = $null
    $vram = $null
    $systemCpu = $null
    try {
        $engines = @(Get-CimInstance Win32_PerfFormattedData_GPUPerformanceCounters_GPUEngine -Filter "Name LIKE 'pid_${TargetProcess}_%'" -ErrorAction Stop | Where-Object { $_.Name -match "^pid_${TargetProcess}_" })
        if ($engines.Count -gt 0) { $gpu = [double](($engines | Measure-Object -Property UtilizationPercentage -Maximum).Maximum) }
        $memory = @(Get-CimInstance Win32_PerfFormattedData_GPUPerformanceCounters_GPUProcessMemory -Filter "Name LIKE 'pid_${TargetProcess}_%'" -ErrorAction Stop | Where-Object { $_.Name -match "^pid_${TargetProcess}_" })
        if ($memory.Count -gt 0) { $vram = [double](($memory | Measure-Object -Property DedicatedUsage -Sum).Sum) / 1MB }
    } catch {}
    try { $systemCpu = [double](Get-CimInstance Win32_PerfFormattedData_PerfOS_Processor -Filter "Name='_Total'").PercentProcessorTime } catch {}
    @{
        interval_seconds = $elapsed
        cpu_percent = [math]::Min(100, [math]::Max(0, 100 * ($cpuTime - $previousCpu) / $elapsed / $LogicalCpus))
        working_set_mib = $game.WorkingSet64 / 1MB
        private_mib = $game.PrivateMemorySize64 / 1MB
        gpu_percent = $gpu
        dedicated_gpu_mib = $vram
        system_cpu_percent = $systemCpu
    } | ConvertTo-Json -Compress
    $previousCpu = $cpuTime
    $previousTime = $now
}
