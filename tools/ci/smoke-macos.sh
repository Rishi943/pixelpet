#!/usr/bin/env bash
# Start PixelPet on the CI Mac and screenshot the desktop.
#   smoke-macos.sh [BINARY]
# Writes empty-mac.png, shot-mac.png (checked) and menu-mac.png (for eyes).
set -euo pipefail
bin=${1:-./target/debug/PixelPet}
here=$(cd "$(dirname "$0")" && pwd)
# settings in a temp home, so the run never touches a real config
export HOME=$(mktemp -d)
# not a first launch: a first launch opens the pet maker over the desktop
mkdir -p "$HOME/Library/Application Support/PixelPet"
echo "pet = alex" > "$HOME/Library/Application Support/PixelPet/settings.txt"

screencapture -x empty-mac.png
PIXELPET_PIN=stand PIXELPET_EXIT_AFTER=8 "$bin" &
pet=$!
sleep 5
screencapture -x shot-mac.png
wait $pet
echo "PixelPet exited with $?"

# the right-click menu, opened without and with keyboard focus
for kind in menu menu-focus; do
  PIXELPET_PIN=stand PIXELPET_SCRIPT="2:$kind" PIXELPET_EXIT_AFTER=6 "$bin" &
  pet=$!
  sleep 4
  screencapture -x "$kind-mac.png"
  wait $pet
  echo "PixelPet ($kind) exited with $?"
done

# a first launch, for a person to look at: the pet maker opens by itself
HOME=$(mktemp -d) PIXELPET_EXIT_AFTER=6 "$bin" &
pet=$!
sleep 4
screencapture -x maker-mac.png
wait $pet
echo "PixelPet (first launch) exited with $?"

# control: can screencapture see another app's window here at all?
open -a Calculator || true
sleep 3
screencapture -x control-mac.png || true
osascript -e 'quit app "Calculator"' || true

# screencapture writes 8-bit RGBA PNGs at the display's pixel size
python3 "$here/check_shot.py" shot-mac.png --empty empty-mac.png
