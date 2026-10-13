# Snipping Tool UL 0.3.17

Completed capture previews now request foreground focus after their first frame is presented. Previously, this path displayed the editor without activating it, allowing it to remain behind the application being captured. The request is made once in `open_preview`, after the capture result and initial clipboard attempt arrive. Ordinary redraws never request foreground focus.

The 0.3.16 pre-capture minimize/cloak and early-reopen guards are retained. The preview remains hidden until the completed capture reaches the UI thread. No always-on-top setting, polling, fixed delay, resident worker or new dependency is added.

## Verification

- All-target tests run headlessly, with generated input and workspace-isolated configuration/temporary files.
- The native foreground request is guarded by visibility and minimized state. First-frame presentation precedes activation, including the copy-failure preview. An independent Luna review confirmed the placement and identified synthetic Alt input inside the Windows library focus helper; the final implementation instead raises the normal window once and calls `SetForegroundWindow` directly, without sending keys.
- Windows may deny foreground activation under its [foreground policy](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setforegroundwindow). In that case the app requests taskbar attention once. It does not retry, stay always on top, inject input or change OS settings.
- Focus is deliberately not exercised by the private desktop probe. Source ordering, compilation and regression checks are separate from a live Windows foreground acceptance test.
- The private presenter probe now shows its synthetic About window directly, avoiding `About::show` and its library focus helper. This prevents that probe from requesting activation or synthetic Alt input on the working desktop.
- Portable builds remap machine-specific paths; packaging verifies source identity, binary CLI/version metadata, license notices and ZIP hashes. Quiet installation preserves existing settings, prior executables and running app processes.

Native foreground behavior across applications/full-screen windows still needs acceptance in an isolated Windows session. No working-desktop focus, capture, clipboard, tray or hotkey tests are performed. This local update is separate from the published 0.3.15 preview.
