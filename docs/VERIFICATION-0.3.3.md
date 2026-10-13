# 0.3.3 supplied app and tray icons

The supplied teal bird/scissors ICO is embedded directly in the Windows executable, preserving all nine authored sizes: 16, 20, 24, 32, 40, 48, 64, 128 and 256 pixels. The cached window icon uses the exact decoded 64px ICO frame. The 256px documentation image is the original ICO PNG frame.

The dedicated tray SVG is prepared offline into two 32px straight-alpha pixel buffers. Startup reads Windows' `SystemUsesLightTheme` value once, choosing dark ink for a light taskbar or soft white for a dark taskbar. An unavailable value defaults to white. A later Windows theme change takes effect on the next app start. Tray tint follows the Windows taskbar, independently of the app's selected theme.

Startup performs no SVG parsing, PNG decoding, rasterization or permanent polling for these icons. Raw tray buffers add 8 KiB; window icon caching is preserved. `cargo run --example logo_assets -- target/logo-assets` regenerates resources headlessly into a separate output directory and refuses to replace differing outputs. The original user-provided base-folder files are preserved.

175 headless Rust tests, formatting and five mocked installer scenarios pass. Actual 16/32px tray and 64px app renders were inspected on light/dark authored backgrounds. Read-only Windows resource extraction confirms all nine embedded icon image payloads match the original ICO and all three embedded raw buffers match prepared assets. Windows' resource compiler normalizes the ICO plane metadata; image bytes are unchanged. The supplied SVGs and copied ICO also match their originals.

Portable packaging checks safe CLI commands, archive hashes and public identity scans. Tests create no app/tray windows and perform no screenshot capture, clipboard write or upload. Versioned installation verifies the binary and Start shortcut while preserving running processes.

Native shell presentation and end-to-end capture/paste, mixed-DPI, timer/hotkey, upload and total-process performance acceptance remain separate. No new performance benchmark or GitHub publication is claimed. See [0.3.0 verification](VERIFICATION-0.3.0.md) for those limits.
