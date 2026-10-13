# Snipping Tool UL 0.2.1 verification

2026-10-12. Adds user-requested Windows takeover through Settings and an opt-in installer checkbox. Prior capture/render evidence remains in VERIFICATION-0.2.0.md.

- 164 headless Rust tests passed with no failures. New tests cover preview-only CLI defaults, explicit replacement flags, the interactive install gate and action geometry.
- Five PowerShell sandbox scenarios passed: preview, alongside installation, exact-package takeover/idempotence, unrelated shortcut conflict and removal failure. Real Appx commands were mocked and Start shortcuts stayed inside a test folder. Failure retained UL and did not write a success state.
- Installer script syntax checked. The actual Windows tab drawing function rendered and was visually inspected headlessly. Installer dialog native appearance has not been exercised; it is an opt-in WinForms setup UI and adds no application idle work.
- Shared integration installs the executable and copyright/font/icon/dependency notices to a versioned current-user folder. It compares source/destination hashes, preserves differing existing files and unrelated shortcuts, then removes only the exact Microsoft.ScreenSketch package registration for the current user. No machine-wide removal, provisioning changes, startup toggle or hotkey remapping occurs.
- Settings requires a clear Yes/No confirmation and runs installation on a worker; the event loop receives its completion result. CLI defaults to preview. The installer replacement checkbox defaults off. Restore opens Microsoft's official Store page rather than claiming an automatic rollback.

The user explicitly requested opening the app on the active desktop. The 0.2.0 UI was launched and its process/main window confirmed. An earlier combined install/register/launch tool call was automatically rejected before execution with no specific reason; the standalone launch succeeded. Installation and current-account takeover are separately verified in the local installation audit when applied.

No screenshot captures, clipboard writes, uploads or GUI test fixtures were executed on the working desktop. The opened application is user-directed use, not automated capture testing. Live capture/paste/upload, mixed-monitor native behavior, active backdrop visual acceptance and full-product memory/capture latency remain unverified.
