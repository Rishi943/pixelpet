# Add PixelPet to the Start menu (and so to Windows search) for this user:
# writes "PixelPet.lnk" in %APPDATA%\Microsoft\Windows\Start Menu\Programs,
# pointing at the exe, with PixelPet.ico from beside the exe as its icon
# (make it with: cargo run --release --example make_icon).
# To remove it, delete that one .lnk file.
# Usage: powershell -File tools/start-menu.ps1 [-Exe target/release/PixelPet.exe]
param([string]$Exe = 'target/release/PixelPet.exe')

$ErrorActionPreference = 'Stop'
$exePath = (Resolve-Path $Exe).Path
$dir = Split-Path $exePath
$ico = Join-Path $dir 'PixelPet.ico'
$lnk = Join-Path ([Environment]::GetFolderPath('Programs')) 'PixelPet.lnk'

$s = (New-Object -ComObject WScript.Shell).CreateShortcut($lnk)
$s.TargetPath = $exePath
$s.WorkingDirectory = $dir
$s.Description = 'PixelPet desktop pet'
if (Test-Path $ico) { $s.IconLocation = "$ico,0" } else { Write-Output "no $ico yet: the shortcut gets the default icon" }
$s.Save()
Write-Output "wrote $lnk -> $exePath"
