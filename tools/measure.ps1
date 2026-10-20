# Measure PixelPet against the performance budget (spec section 9).
# For each of sleep and walk: start the exe with that anim pinned, sample its
# CPU time and working set every second, let it exit by itself, repeat.
# Reports the median of the runs. ASCII only (Windows PowerShell 5 reads
# scripts as ANSI).
# Usage: powershell -File tools/measure.ps1 -Exe target/release/PixelPet.exe [-Seconds 60] [-Runs 3]
param(
    [Parameter(Mandatory = $true)][string]$Exe,
    [int]$Seconds = 60,
    [int]$Runs = 3
)

$ErrorActionPreference = 'Stop'
$exePath = (Resolve-Path $Exe).Path
# only one PixelPet can run; a second copy would exit at once and measure nothing
if (Get-Process -Name PixelPet -ErrorAction SilentlyContinue) {
    Write-Output 'PixelPet is already running: quit it from its menu first, then measure.'
    exit 1
}
$cores = [Environment]::ProcessorCount
$warmup = 3

function Median($xs) {
    $s = @($xs | Sort-Object)
    $n = $s.Count
    if ($n -eq 0) { return [double]::NaN }
    if ($n % 2 -eq 1) { return [double]$s[($n - 1) / 2] }
    return ([double]$s[$n / 2 - 1] + [double]$s[$n / 2]) / 2
}

$budget = @{ sleep = 0.5; walk = 1.5 }
$rows = @()
foreach ($anim in @('sleep', 'walk')) {
    $cpuRuns = @()
    $memRuns = @()
    for ($r = 1; $r -le $Runs; $r++) {
        $env:PIXELPET_PIN = $anim
        $env:PIXELPET_EXIT_AFTER = [string]($warmup + $Seconds + 3)
        $p = Start-Process -FilePath $exePath -PassThru
        $env:PIXELPET_PIN = $null
        $env:PIXELPET_EXIT_AFTER = $null
        Start-Sleep -Seconds $warmup
        $p.Refresh()
        $cpu0 = $p.TotalProcessorTime.TotalSeconds
        $t0 = Get-Date
        $peak = 0
        for ($i = 0; $i -lt $Seconds; $i++) {
            Start-Sleep -Seconds 1
            $p.Refresh()
            if ($p.HasExited) { break }
            if ($p.WorkingSet64 -gt $peak) { $peak = $p.WorkingSet64 }
        }
        $p.Refresh()
        $cpu1 = $p.TotalProcessorTime.TotalSeconds
        $wall = ((Get-Date) - $t0).TotalSeconds
        $pct = ($cpu1 - $cpu0) / $wall / $cores * 100
        $cpuRuns += $pct
        $memRuns += $peak / 1MB
        Write-Output ("  {0} run {1}: CPU {2:N3} %  peak working set {3:N1} MB  ({4:N1} s)" -f $anim, $r, $pct, ($peak / 1MB), $wall)
        if (-not $p.WaitForExit(15000)) { Write-Output "  WARNING: PixelPet did not exit by itself (pid $($p.Id))" }
    }
    $rows += [pscustomobject]@{ Anim = $anim; CpuPct = (Median $cpuRuns); MemMB = (Median $memRuns) }
}

$sizeMB = (Get-Item $exePath).Length / 1MB
Write-Output ""
Write-Output ("{0} logical cores, {1} runs x {2} s each, medians:" -f $cores, $Runs, $Seconds)
Write-Output ""
Write-Output "  anim    CPU %    budget   memory MB  budget"
$allPass = $true
foreach ($row in $rows) {
    $cpuOk = $row.CpuPct -le $budget[$row.Anim]
    $memOk = $row.MemMB -le 30
    if (-not ($cpuOk -and $memOk)) { $allPass = $false }
    Write-Output ("  {0,-6} {1,7:N3}   <= {2,-4}  {3,8:N1}    <= 30   {4}" -f $row.Anim, $row.CpuPct, $budget[$row.Anim], $row.MemMB, $(if ($cpuOk -and $memOk) { 'PASS' } else { 'FAIL' }))
}
$sizeOk = $sizeMB -le 6
if (-not $sizeOk) { $allPass = $false }
Write-Output ("  file   {0:N2} MB (budget <= 6 MB)  {1}" -f $sizeMB, $(if ($sizeOk) { 'PASS' } else { 'FAIL' }))
Write-Output ""
Write-Output $(if ($allPass) { 'PASS' } else { 'FAIL' })
