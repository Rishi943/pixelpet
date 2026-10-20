# PixelPet gotchas

Traps that cost real time, one each, with the fix.

- **PixelPet (Windows layered windows): a winit window fed by `UpdateLayeredWindow` gets a clipping region and a hit-test frozen at an old size.** Cost: an evening — clicks died after the first drag, and L/XL showed only the ears. Fix (in `src/window/windows.rs`): clear the region with `SetWindowRgn(hwnd, 0, 0)` on every resize, and answer `WM_NCHITTEST` with HTCLIENT via a window subclass. `tests/window_hit.rs` reproduces both.
- **PowerShell variables ignore case: a `$w` loop variable overwrote the `$W` width parameter.** Cost: one round of 2-px-wide screenshots. Never reuse a parameter's name in any case.
- **Git Bash rewrites `/IM` in `taskkill /IM x.exe` into a path.** Use `taskkill //IM x.exe` (and `//F`) from the `!` prompt.
