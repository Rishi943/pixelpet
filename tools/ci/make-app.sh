#!/usr/bin/env bash
# Package a universal PixelPet binary as PixelPet.app inside PixelPet.dmg.
#   make-app.sh UNIVERSAL_BINARY VERSION
set -euo pipefail
bin=$1
version=$2
here=$(cd "$(dirname "$0")" && pwd)
rm -rf dist && mkdir -p dist/PixelPet.app/Contents/{MacOS,Resources}
app=dist/PixelPet.app/Contents
cp "$bin" "$app/MacOS/PixelPet"
sed "s/@VERSION@/$version/g" "$here/Info.plist" > "$app/Info.plist"
cargo run -q --release --example make_icon -- --pngs dist/icons
iconutil -c icns dist/icons/PixelPet.iconset -o "$app/Resources/PixelPet.icns"
plutil -lint "$app/Info.plist"
# ad-hoc signature: still "unsigned" (no developer ID), but a quarantined
# download then shows the "Open Anyway" warning, not "PixelPet is damaged"
codesign --force --sign - dist/PixelPet.app
codesign --verify --strict --verbose=2 dist/PixelPet.app
mkdir -p dist/dmg && cp -R dist/PixelPet.app dist/dmg/ && ln -s /Applications dist/dmg/Applications
hdiutil create -volname PixelPet -srcfolder dist/dmg -ov -format UDZO PixelPet.dmg
echo "made PixelPet.dmg"
