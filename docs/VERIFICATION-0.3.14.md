# 0.3.14 preview update

This package incorporates the coordinated reliability fixes, direct Windows OCR, default frozen selection, annotated-rendering optimizations and History memory/cleanup pass. Changes and measured component limits are described in [History memory](HISTORY-MEMORY-2026-10-13.md), [rendering optimizations](OPTIMIZATION-PASS-2026-10-13.md), [frozen capture](FROZEN-CAPTURE.md), [frozen performance](FROZEN-PERFORMANCE.md) and the changelog.

The release binary is built with thin LTO and stripped debug information. The locked/offline all-target suite passed 312 tests, zero failures and three ignored manual diagnostics, with no compiler warnings. Quiet installer integration tests passed in PowerShell 7 and Windows PowerShell 5.1 with filesystem, Start shortcuts, startup registry and Appx commands isolated or mocked. The preceding History pass separately verified 256 memory-DIB draw/drop cycles and 500 reload/cancel requests plus 200 crop/render cycles with stable handles.

Package acceptance checks cover binary/public-file identity, safe CLI exits, Windows icon resources, embedded app/tray pixels, archive entry hashes and current-user installation readback. Tests create no working-desktop UI and perform no capture, clipboard, hotkey registration, tray, notifications or live uploads. Source-rendered thumbnail and dark/light previews were inspected in the preceding component passes; no native visual acceptance is claimed.

Quiet installation uses a new versioned folder and updates the existing owned Start shortcut. The installer carries an existing owned startup opt-in forward and preserves an opt-out or custom command. No running application is terminated or launched. Existing configuration and History files are preserved. Quit UL and reopen from Start when convenient to load this version.

Full native capture/paste, mixed-monitor DPI, timers/hotkeys, provider behavior, full-app latency and long-session memory remain the deferred isolated Windows acceptance gate. Component measurements are not total-app RAM or a measured comparison with Windows Snipping Tool. No Sandbox setup, Git commit, push, tag, release or public upload is performed.
