# Snipping Tool UL 0.3.23: exact region release

Region snips now end exactly where the mouse button was released. Earlier versions read the live cursor when the release message was handled, so a pointer still moving after release could enlarge the snip. Start on login also stays on the newest installed version when an older running copy saves Settings.

## Verification

- 358 locked all-target automated tests pass, zero failures, including new tests for signed mouse-message coordinates and nine startup-registration cases.
- Native region capture on Windows 11 with three 3840 x 2160 monitors at 150% scaling, against a known tile pattern: release followed 1 ms later by a 300 px pointer move gave the exact 1280 x 720 region in 15 of 15 trials (0.3.22: 1 of 10). Normal drags were exact in 10 of 10, with release-to-clipboard and release-to-editor times unchanged.
- Measured comparison with Microsoft Snipping Tool 11.2608.41.0 (on 0.3.22, which shares this capture path): [performance results](PERFORMANCE.md#microsoft-stock-snipping-tool-comparison).
- Remapped Windows build, formatting, source and package identity scans and archive-entry hashes checked before publication.

Mixed-scale monitor layouts, live uploads, OCR comparison and long-session memory remain open.
