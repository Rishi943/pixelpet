#!/usr/bin/env bash
# Package a PixelPet binary as PixelPet-x86_64.AppImage. Nothing else is
# bundled: winit loads the system's X11 libraries at run time.
#   make-appimage.sh BINARY
set -euo pipefail
bin=$1
here=$(cd "$(dirname "$0")" && pwd)
rm -rf AppDir && mkdir -p AppDir/usr/bin
cp "$bin" AppDir/usr/bin/PixelPet
cp "$here/pixelpet.desktop" AppDir/pixelpet.desktop
cargo run -q --release --example make_icon -- --pngs dist-icons
cp dist-icons/pixelpet.png AppDir/pixelpet.png
cat > AppDir/AppRun <<'RUN'
#!/bin/sh
exec "$(dirname "$(readlink -f "$0")")/usr/bin/PixelPet" "$@"
RUN
chmod +x AppDir/AppRun AppDir/usr/bin/PixelPet
# appimagetool release 13 (the last numbered one), run without FUSE
curl -fsSL -o appimagetool https://github.com/AppImage/AppImageKit/releases/download/13/appimagetool-x86_64.AppImage
chmod +x appimagetool
ARCH=x86_64 ./appimagetool --appimage-extract-and-run AppDir PixelPet-x86_64.AppImage
echo "made PixelPet-x86_64.AppImage"
