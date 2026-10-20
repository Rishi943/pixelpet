# Run PixelPet with one anim pinned, take a region screenshot after a wait,
# and let the program exit by itself (PIXELPET_EXIT_AFTER).
# Usage: powershell -File tools/shoot.ps1 -Exe target/release/PixelPet.exe -Pin walk -Out walk.png [-Wait 3] [-X 0 -Y 0 -W 800 -H 600]
param(
    [Parameter(Mandatory = $true)][string]$Exe,
    [Parameter(Mandatory = $true)][string]$Pin,
    [Parameter(Mandatory = $true)][string]$Out,
    [double]$Wait = 3,
    [int]$X = -1,
    [int]$Y = -1,
    [int]$W = -1,
    [int]$H = -1
)

$env:PIXELPET_PIN = $Pin
$env:PIXELPET_EXIT_AFTER = [string]($Wait + 2)
$p = Start-Process -FilePath $Exe -PassThru
Start-Sleep -Milliseconds ([int]($Wait * 1000))
& powershell -NoProfile -File (Join-Path $PSScriptRoot 'screenshot.ps1') -Out $Out -X $X -Y $Y -W $W -H $H
if (-not $p.WaitForExit(10000)) { Write-Output "WARNING: PixelPet still running after its exit time (pid $($p.Id))" }
$env:PIXELPET_PIN = $null
$env:PIXELPET_EXIT_AFTER = $null
