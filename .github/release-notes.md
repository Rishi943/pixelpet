PixelPet is a small pixel pet that lives on your desktop. It walks along the bottom of the screen, naps, and can be petted, played with and fed. One file, no installer, no account.

**New in this version: updates.** PixelPet checks for a new version when it starts and once a day. When there is one, the right-click menu says "Update to v…": click it and PixelPet downloads the update, checks it, and restarts. That check is the only time it goes online. If you have v0.1.0, download this version once by hand; after that it updates itself.

**Downloads**

- **Windows:** `PixelPet.exe`. Run it from anywhere.
- **macOS (Intel and Apple Silicon):** `PixelPet.dmg`. Open it and drag PixelPet to Applications.
- **Linux:** `PixelPet-x86_64.AppImage`. Make it runnable (`chmod +x PixelPet-x86_64.AppImage`), then run it. It needs an X11 desktop or a Wayland one with XWayland, and the X keyboard library every desktop has (`libxkbcommon-x11`).

**The first launch shows a warning, once**

PixelPet isn't signed yet, so each system asks before running it the first time:

- **Windows:** "Windows protected your PC". Click **More info**, then **Run anyway**.
- **macOS:** "PixelPet can't be opened" (or "Apple could not verify…"). Click **Done**, open **System Settings → Privacy & Security**, scroll down and click **Open Anyway** next to PixelPet, then confirm. On macOS 14 and older, right-clicking PixelPet in Applications and choosing **Open** works too. Run it from Applications, not from the dmg, or launch on startup can't be turned on.
- **Linux:** no warning.

The first time it runs, PixelPet opens a pet maker: pick an animal and a breed, shape and colour them, and give them a name. Make more pets any time with "Make a pet..." in the right-click menu.

Right-click the pet for its menu: which pet, size, opacity, what happens over fullscreen apps, launch on startup, updates, and quit.
