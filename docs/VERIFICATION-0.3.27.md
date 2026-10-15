# Snipping Tool UL 0.3.27: copy state, drag out, same area again, hide private info

New: the Copy button tracks the clipboard sequence number to show whether the clipboard still holds the image as shown; press-and-drag on Copy (or Ctrl+drag on a pin) drops the image as a PNG through the shell's own drag and drop; a repeat shortcut (capture shortcut plus Alt) captures the previous area directly; the picker shows the selection size; Tools > Hide private info pixelates emails, long numbers, IP addresses and key-like tokens found by Windows OCR as one undo step; Space+click-drag pans.

## Verification

- 396 locked all-target automated tests pass, including headless tests for clipboard state, the repeat shortcut (every capture-shortcut shape, never a Windows combination), size-label placement, drag-file naming and cleanup, the redaction rules (sample text that must and must not be hidden, 2,000 random OCR lines) and the editor drawn at extreme zoom in windows down to 1 x 1. Clippy reports the same warning count as 0.3.26.
- Five independent code reviews by area (editor input, hotkeys and capture, clipboard and drag-out, OCR and redaction, rendering and timers) plus three follow-up reviews; every confirmed finding was fixed before release.
- Hidden-desktop presentation probe: 60 open and close cycles in both themes on a private desktop; memory growth identical to 0.3.26.
- Live tests on Windows 11 with three 4K monitors at 150%, nobody using the PC, alternating 0.3.26 and this build in 3 blocks of 15 snips: release to clipboard 41.9 ms vs 49.9 ms, hotkey to selection screen 124 ms vs 125 ms, release to editor 109 ms vs 109 ms, peak working set 202 MB for both. All 90 snips pixel-exact.
- Not verified natively: dragging into specific apps, Space panning, the tray item and Hide private info on real screenshots have no automated coverage and were only tried by hand on one PC; mixed-scaling monitors remain untested.
- Remapped Windows build, formatting, source and package identity scans and archive-entry hashes checked before publication.
