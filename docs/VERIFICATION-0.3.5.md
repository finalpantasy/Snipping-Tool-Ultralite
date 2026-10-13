# 0.3.5 publication readiness

Task Manager's process description and product name are now **Snipping Tool UL**, verified through the release executable's Windows version resource. The executable filename remains `snipping-tool-ul.exe`. Existing running processes keep their old metadata until restarted.

Options and the editor gear menu now group Capture, Sharing and App controls, show explicit On/Off badges, show the image host separately and offer a distinct **Quit Snipping Tool UL** command. Menu surfaces are opaque for readable text while the workspace remains frosted. Settings > General exposes both closing choices: **Keep in tray** (default) and **Quit completely**. The home and editor X honor the saved choice; settings and countdown dialogs retain their own close/cancel behavior. Older configurations preserve close-to-tray behavior.

The local headless suite contains 178 tests, including config migration/round-trip, close-setting hit targets, and mouse/keyboard access to Quit. Actual client renders cover both themes, minimum editor dimensions, 150/200 percent DPI, focus states and both close-setting choices. Synthetic previews do not show the native frame. Installer scenarios mock Appx and use isolated files; no real uninstall is part of those tests. Clippy is now included in the Windows CI workflow; nonfatal style warnings remain. CI action commit references were verified against their official repositories; remote CI has not run because nothing has been pushed.

## Dependency check

The locked registry graph was queried by package name/version through the [OSV API](https://google.github.io/osv.dev/post-v1-querybatch/), which includes [RustSec advisories](https://rustsec.org/). Rustls was updated to 0.23.45 to address [RUSTSEC-2026-0285](https://rustsec.org/advisories/RUSTSEC-2026-0285.html). Updating notify-rust to 4.18.2 removes the old Windows quick-xml notification dependency affected by [RUSTSEC-2026-0194](https://rustsec.org/advisories/RUSTSEC-2026-0194.html) and 0195. Memmap2 was updated to 0.9.11 to address [RUSTSEC-2026-0186](https://rustsec.org/advisories/RUSTSEC-2026-0186.html).

No known vulnerability/unsoundness finding remains in the 219-package resolved Windows registry graph at the check time. Rustybuzz and ttf-parser still have unmaintained advisories through the font/render libraries; these are documented maintenance risks rather than a clean all-platform audit. Non-Windows lockfile dependencies still have advisories and are outside this Windows release's support claim. The source declares Rust 1.88 to match the maximum declared Windows dependency requirement. Dependency notices cover every resolved Windows registry package and were regenerated from the new graph. Advisory coverage changes over time and is not proof that software is secure.

## Boundaries

The remapped release, resource/embedded-icon checks, CLI validation, package hashes and public identity checks must pass before installation. Synthetic CPU rendering measurements exclude capture, clipboard, native presentation and uploads; retained pixel buffers are not total process memory. Private-desktop presentation checks never switch the input desktop and perform no capture, clipboard, hotkey, tray, notification or upload operations.

Real native working README media remains required by the publication policy. Real capture/paste, mixed-monitor behavior, timer/hotkey registration, upload success/retry, native close/exit behavior and full-app memory/latency remain isolated Windows acceptance work. No Microsoft Snipping Tool comparison is claimed. See [publication checklist](PUBLICATION.md).
