# Snipping Tool UL 0.3.22: automatic canvas

Annotations can extend beyond every image edge. White space grows around them and contracts when they move inward, resize or are deleted. Original image pixels and pointer mapping stay steady. Undo/redo restore canvas bounds. Copy, Save, Upload and Pin export the complete canvas; raw OCR input remains the original image.

The bug test corrected fractional pixelation and spotlight source-region rounding, expanded-pin origin and silent export-preparation rejection. Details: [canvas bug test](CANVAS-BUG-TEST-2026-10-14.md).

## Verification

- 348 locked/offline all-target automated tests pass, zero failures, three existing manual tests ignored.
- Twenty canvas tests cover all stroked edges, live drawing/text, resize/move/delete/undo/redo, alpha, crop and monitor offsets, empty source and allocation limits.
- Seventy-two alpha/effect scenarios preserve source pixels across expansion. One hundred eighty cached/fresh/offscreen/zoom/pan transitions match and stay within the viewport memory budget.
- The OCR panic-cleanup test uses a ten-second fixture deadline to avoid confusing loaded CI scheduling with cleanup failure; the production thirty-second timeout is unchanged. Final all-target tests use normal parallel scheduling, matching CI.
- Current light/dark, 100%/150% editor renders and full-resolution exports were generated headlessly and representative outputs inspected.
- Versioned remapped Windows build, formatting, whitespace, mocked installer scenarios, source/package identity and archive-entry hashes checked before publication. No dependency versions changed.

Native mouse, clipboard and pin-window acceptance remains pending an isolated Windows session. This publication does not install the release or alter running processes/settings.
