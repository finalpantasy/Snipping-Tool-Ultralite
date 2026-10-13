# 0.3.12 background audit and performance pass

Capture History is the first item under Capture in Options. Its badge says **Currently off** or **Currently on**, reflecting the saved recording preference in both the launcher and editor. Opening history does not enable recording. Dark/teal frost remains the default; light mode remains available.

## Verified fixes

- History thumbnails load on a worker, with cancellation on refresh/close and redraw on completion. Reopening the menu refreshes an existing gallery. PNG/JPEG/WebP history files retain their actual formats. Writes use unique temporary files and publish only after completion; save/list/prune/clear are serialized within the process.
- Settings save failures retain edits and display an explanation instead of silently closing. A startup-registration failure explicitly says that the other settings were saved. Startup registry handles close on every error path.
- A failed hotkey replacement retains the working registration and reports the actual binding. Cancelling a capture no longer supersedes a pending clipboard result. The tray's one-off window capture restores the configured capture mode.
- The event loop selects the earliest countdown, capture, toast, render or folder-picker deadline. Idle handling cannot replace that deadline with an indefinite wait.
- Unchanged annotated viewports reuse existing composed pixels, without adding another image buffer. Comparison metadata is capped at 256 KiB; large freehand marks bypass it. Selection, edits, zoom, text entry, theme and eyedropper invalidate or bypass the cache.
- Pixelation export directly writes averaged blocks and bounds oversized annotation geometry to the source. OCR and pin encoding borrow pixels instead of cloning the complete image. OCR cleans partial temporary files and resolves the Windows PowerShell executable directly. No-setup uploads have bounded 15-second connect, 30-second read, 120-second write and 180-second total timeouts; cooldown and confirmation behavior remain intact.

## Measurements

Release-mode synthetic fixture: 3840 x 2160 image, 24 arrows and a 3,000-point pencil stroke, with a 1410 x 918 viewport at 150% DPI. Viewport numbers are medians of 31 warmed draws; export numbers are medians of five runs. The machine remained in use, so these are indicative CPU measurements, not end-to-end latency guarantees.

| Operation | Before | After |
| --- | ---: | ---: |
| Unchanged edited viewport | 3.662 ms | 0.031 ms |
| Full 4K pixelation export, 12-pixel blocks | 42.733 ms | 19.339 ms |
| Full 4K pixelation export, 2-pixel blocks | 618.261 ms | 42.520 ms |

Renderer-owned retained data in this fixture changed from 3,875,328 to 3,900,728 bytes, an additional 25,400 bytes of annotation metadata. This is not total app memory. Existing pixelation-grid storage remains capped at 1 MiB.

## Checks and boundaries

224 locked all-target tests pass, including fresh-versus-cached pixel comparisons, edit invalidation, metadata limits, exact pixelation block means, oversized regions, concurrent history saves, actual history formats, settings failure/retry, timer priority and menu hit targets. Formatting and whitespace checks pass; Clippy completes with nonblocking style warnings. Five installer scenarios pass against mocks. Storage/config tests use temporary roots.

50 material and 16 history fixtures were generated using production drawing code and synthetic pixels. Dark Options/off, light Options/on and the Settings error layout were visually inspected. These are headless renders, not native workflow screenshots.

The guarded private-desktop probe refused the normal desktop before creating GUI resources, then passed ten create/present/close cycles on a private Win32 desktop. Both themes accepted native caption styling and used the glass presenter. The harness verified the input desktop was unchanged. It performed no capture, clipboard, hotkey, tray, notification or upload operations. Private-desktop presentation timings and working sets are not representative full-app performance measurements.

The remapped shipping executable is checked for version/help/invalid-region handling, icon resources, public identity, archive content hashes, installed file hash, process description and shortcut/icon target. Updating preserves running processes and does not launch the app. Quit UL and reopen it when convenient to load this build.

The current dependency query covers 468 locked registry packages, of which 218 are resolved for Windows. Windows findings are unmaintained rustybuzz/ttf-parser versions; no vulnerability or unsoundness finding was returned for that graph. Advisory coverage is not a security guarantee. Notice headings cover the Windows graph. Rust 1.89 is required by notify-rust 4.18.2; the minimum toolchain itself was not exercised. Other-platform advisories remain outside this Windows pass.

See [audit coverage and remaining work](AUDIT-0.3.12.md). No commit, push or public release was performed.
