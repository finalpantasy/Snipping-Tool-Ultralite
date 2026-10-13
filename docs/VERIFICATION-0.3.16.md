# Snipping Tool UL 0.3.16

The canvas and UL control windows now minimize and hide before capture. Both New and tray timers prepare the windows before starting the countdown. The existing compositor flush remains immediately before frozen desktop acquisition for region, window and full-screen selection. Already hidden windows are left alone so preparation cannot show them again.

The reported fading window revealed that hiding alone is insufficient for the intended workflow. Preparation disables transitions on UL's own window and applies the native DWM cloak before minimization. Explicit restoration clears that cloak. Canvas rendering and Home reopen events are blocked while a countdown, capture grace period or picker is active. No fixed sleep was added. Microsoft's [DWM window attributes](https://learn.microsoft.com/en-us/windows/win32/api/dwmapi/ne-dwmapi-dwmwindowattribute) distinguish transition control and cloaking; [DwmFlush](https://learn.microsoft.com/en-us/windows/win32/api/dwmapi/nf-dwmapi-dwmflush) synchronizes a compositor presentation rather than serving as an animation-completion delay.

Cancelling a timer restores the preceding preview and its annotations. Settings and History can be reopened after being hidden. The tray tooltip and Capture screenshot menu show the successfully registered hotkey, including fallback and unavailable states; Settings changes update the hint.

## Verified

- 321 locked, offline all-target headless tests passed, zero failures; three manual diagnostics remained ignored. Test configuration and temporary files were isolated inside the workspace. An initial run encountered temporary-file access restrictions; its failure log was retained, then the isolated run passed.
- 60 guarded synthetic presentation cycles on a private Win32 desktop verify actual minimized/hidden state, repeated preparation without revealing the window, and visibility restoration. The extended probe reads the native application-cloak bit, submits a late synthetic paint while hidden, then checks that the cloak persists and clears on explicit restoration. The input desktop must not change. The probe performs no screen capture, clipboard, hotkey, tray, notification or upload operations.
- Mocked installer integration, formatting, diff checks and public source identity checks passed.
- A portable Windows x64 release was built with machine-specific paths remapped. Packaging checks verify executable metadata, CLI version, license notices, file hashes and public identity.

## Limits

The private probe verifies window behavior, not a complete live screenshot workflow. Actual exclusion from screenshots, timer cancellation with real annotation interactions and the native tray tooltip/menu still need isolated end-to-end acceptance. No working-desktop GUI or capture tests were run. Existing capture, automatic clipboard copy and preview behavior are preserved by the shared capture path.

This local update is separate from the published 0.3.15 preview. No public release or repository update is performed by the build or installer. Quiet installation preserves existing settings, prior executables and running app processes; reopening UL loads the new version.
