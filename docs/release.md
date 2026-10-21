# Releasing PixelPet

A tag `v*` runs `.github/workflows/release.yml`: it builds the three downloads, starts each on a CI machine and screenshots it, and makes a **draft** release holding them with `docs/release-notes.md` as its text. Publishing the draft is done by hand.

| File | Built on | Checked on CI |
|---|---|---|
| `PixelPet.exe` | windows-latest, MSVC, static C runtime | size ≤ 6 MB; no `VCRUNTIME140.dll` or other C-runtime imports |
| `PixelPet.dmg` | macos-latest, x86_64 + arm64 joined with `lipo` | both architectures present; the bundled app starts and shows the pet and the menu (screenshots) |
| `PixelPet-x86_64.AppImage` | Debian bullseye (old glibc) | starts on Xvfb with and without a compositor; pet on the ground, no box (screenshots) |

Builds are unsigned (spec §10). The release notes explain the one-time first-launch step.

## Still owed before a release counts as done (spec §11.6)

CI machines prove the program starts and draws. They can't prove it feels right. On a **real Mac** and a **real Linux desktop** (X11 and a Wayland session), check each of these and note the date and the machine:

- [ ] The pet stands on the ground: the top of the Dock on macOS, the top of the panel on Linux.
- [ ] Clicks pass through the empty space around the pet, and the pet itself can be clicked.
- [ ] Drag and drop; dropping in mid-air falls back down.
- [ ] Hovering opens the bubble; Pet, Play (fetch) and Feed all work.
- [ ] Right-click opens the menu; every setting changes and is remembered after a restart.
- [ ] Play ends on right-click (macOS reads no Esc key, by design) and on Esc (Linux).
- [ ] A fullscreen video hides the pet with "Hide", and dims it with "Stay visible".
- [ ] Launch on startup survives a log-out and log-in (macOS: run from Applications).
- [ ] Only one PixelPet runs when it's launched twice.
- [ ] No Dock icon (macOS) and no taskbar entry (Linux).
- [ ] macOS: no permission prompt of any kind appears.

As of 2026-10-21 nobody has run these; Rushi has no Mac or Linux machine.
