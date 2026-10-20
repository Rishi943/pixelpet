# Screenshot anims in the real app, for checking likeness against the photos.
# Copies the exe to <OutDir>\app with its own settings.txt (so the user's
# settings are never touched), then for each anim: starts it pinned, takes a
# region screenshot at each wait, and lets it exit by itself.
# Usage: powershell -File tools/anim-shots.ps1 -Exe target/release/PixelPet.exe -Pet alex|alex-the-bun -Anims run,eat -Waits 1.0,1.3 -OutDir <dir> [-X -Y -W -H]
param(
    [Parameter(Mandatory = $true)][string]$Exe,
    [Parameter(Mandatory = $true)][string]$Pet,
    [Parameter(Mandatory = $true)][string]$Anims,
    [string]$Waits = '1.0,1.4,1.8',
    [Parameter(Mandatory = $true)][string]$OutDir,
    [int]$X = 980, [int]$Y = 1060, [int]$W = 600, [int]$H = 380
)
$ErrorActionPreference = 'Stop'
# lists come in as one comma-separated string under powershell -File
$animList = @($Anims -split ',' | ForEach-Object { $_.Trim() } | Where-Object { $_ })
$waitList = @($Waits -split ',' | ForEach-Object { [double]::Parse($_.Trim(), [Globalization.CultureInfo]::InvariantCulture) })
if (Get-Process -Name PixelPet -ErrorAction SilentlyContinue) {
    Write-Output 'PixelPet is already running: quit it from its menu first.'
    exit 1
}
$app = Join-Path $OutDir 'app'
New-Item -ItemType Directory -Force $app | Out-Null
Copy-Item (Resolve-Path $Exe).Path (Join-Path $app 'PixelPet.exe') -Force
"pet = $Pet`nsize = M`nopacity = 100`nfullscreen = stay`nfullscreen_opacity = 100`nautostart = off`n" | Out-File -Encoding ascii (Join-Path $app 'settings.txt')
$shot = Join-Path $PSScriptRoot 'screenshot.ps1'
foreach ($anim in $animList) {
    $env:PIXELPET_PIN = $anim
    $env:PIXELPET_EXIT_AFTER = [string](($waitList | Measure-Object -Maximum).Maximum + 1.5)
    $p = Start-Process -FilePath (Join-Path $app 'PixelPet.exe') -PassThru
    $t0 = Get-Date
    $n = 0
    foreach ($wait in $waitList) {
        $left = $wait - ((Get-Date) - $t0).TotalSeconds
        if ($left -gt 0) { Start-Sleep -Milliseconds ([int]($left * 1000)) }
        $n++
        & powershell -NoProfile -File $shot -Out (Join-Path $OutDir "$Pet-$anim-$n.png") -X $X -Y $Y -W $W -H $H | Out-Null
    }
    if (-not $p.WaitForExit(10000)) { Write-Output "WARNING: still running (pid $($p.Id))" }
    Write-Output "$anim done"
}
$env:PIXELPET_PIN = $null
$env:PIXELPET_EXIT_AFTER = $null
