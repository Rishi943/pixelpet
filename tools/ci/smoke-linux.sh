#!/usr/bin/env bash
# Start PixelPet on a virtual X server and screenshot the desktop.
#   smoke-linux.sh MODE [BINARY]
# MODE: composited (xcompmgr running) or bare (no compositor: shape fallback).
# Writes shot-MODE.png and checks it with check_shot.py.
set -euo pipefail
mode=$1
bin=${2:-./target/debug/PixelPet}
here=$(cd "$(dirname "$0")" && pwd)
bg=3a6ea5
# each mode on its own display, so one run's server can't linger into the next
if [ "$mode" = composited ]; then disp=:99; else disp=:98; fi
export DISPLAY=$disp

Xvfb $disp -screen 0 1920x1080x24 +extension Composite &
xvfb=$!
trap 'kill $xvfb ${comp:-} 2>/dev/null || true' EXIT
for _ in $(seq 50); do xdpyinfo >/dev/null 2>&1 && break; sleep 0.1; done
xdpyinfo >/dev/null
if [ "$mode" = composited ]; then
  # default mode blends alpha; it paints its own grey desktop
  xcompmgr &
  comp=$!
  sleep 1
fi
# xsetroot -solid leaves Xvfb's root black; a root image does show
convert -size 1920x1080 "xc:#$bg" "PNG24:bg.png"
display -window root bg.png
sleep 0.5

# the desktop without the pet, to compare against
import -window root "PNG24:empty-$mode.png"
python3 "$here/check_shot.py" "empty-$mode.png" --uniform

# settings in a temp folder, so the run never touches a real config
export XDG_CONFIG_HOME=$(mktemp -d)
PIXELPET_PIN=stand PIXELPET_EXIT_AFTER=8 "$bin" &
pet=$!
sleep 5
import -window root "PNG24:shot-$mode.png"
wait $pet
echo "PixelPet exited with $?"
python3 "$here/check_shot.py" "shot-$mode.png" --empty "empty-$mode.png"
