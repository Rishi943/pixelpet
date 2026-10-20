# Capture the screen (or a region of it) to a PNG. Physical pixels.
# Usage: powershell -File tools/screenshot.ps1 -Out shot.png [-X 0 -Y 0 -W 800 -H 600]
param(
    [Parameter(Mandatory = $true)][string]$Out,
    [int]$X = -1,
    [int]$Y = -1,
    [int]$W = -1,
    [int]$H = -1
)

Add-Type -AssemblyName System.Drawing
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class PixelPetDpi {
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
    [DllImport("user32.dll")] public static extern int GetSystemMetrics(int i);
}
'@
[PixelPetDpi]::SetProcessDPIAware() | Out-Null

if ($W -le 0 -or $H -le 0) {
    $X = 0; $Y = 0
    $W = [PixelPetDpi]::GetSystemMetrics(0)
    $H = [PixelPetDpi]::GetSystemMetrics(1)
}

$bmp = New-Object System.Drawing.Bitmap $W, $H
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen($X, $Y, 0, 0, (New-Object System.Drawing.Size $W, $H))
if ([System.IO.Path]::IsPathRooted($Out)) { $path = $Out } else { $path = Join-Path (Get-Location).ProviderPath $Out }
$bmp.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose(); $bmp.Dispose()
Write-Output "saved $path ($W x $H at $X,$Y)"
