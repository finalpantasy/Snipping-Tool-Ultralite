# Snipping Tool UL 0.3.26: faster snips

The frozen screen is captured once into a GDI DIB section and painted as it is; only the selected rectangle is converted to RGBA after release. The clipboard is filled before the frozen screen is freed or the editor opens. Hotkey snips skip the pre-capture display-frame wait unless a UL window was hidden or closed within the last second.

## Verification

- All locked all-target automated tests pass, including new tests for cropping and painting from the snapshot (with a negative monitor origin) and for the window-removal grace period.
- Component timings with generated pixels at three 3840 x 2160 monitors: work before the selection screen appears fell from 74 to 88 ms to 24 to 27 ms; work between release and clipboard fell from about 16 ms plus one display frame to about 4.5 ms. A 256-snapshot loop showed no GDI or memory leak.
- Live A/B on Windows 11 with three 4K monitors at 150%, the two builds run back to back on the same busy PC (CPU 76 to 90%): median hotkey to selection screen 141 ms vs 241 ms (0.3.25), release to clipboard 54 ms vs 83 ms, release to editor 94 ms vs 127 ms. 24 of 24 measured snips and 10 of 10 release-then-flick snips were pixel-exact. One warm-up snip was cancelled by user input during the test.
- A code review of the change found that closing the editor, Settings, History, About or Home and pressing the hotkey at once could skip the display-frame wait; fixed before release.
- Remapped Windows build, formatting, source and package identity scans and archive-entry hashes checked before publication.
