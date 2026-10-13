# 0.3.13 About window and isolated-test preparation

The tray's About action previously sent a short Windows notification and ignored notification errors. It did not create a window, so suppressed or missed notifications could make it appear inert. About now opens a small themed native window showing the teal logo, version and license information. A second request restores/focuses the existing About window. Close, X, Escape, Enter and Space dismiss this dialog without quitting UL. Starting capture closes it first; saved theme changes request a redraw.

The window is allocated only on demand and uses the existing event-driven native presenter. Its prepared logo is decoded lazily once and retained at 128 x 128 (64 KiB) for sharp 100-200% scaling. No new dependency, tray polling or persistent repaint loop is added.

225 locked all-target tests pass. The new About rendering test checks the embedded version, physical-resolution title detail, alpha correctness and button bounds through 200% DPI. Formatting, whitespace and Clippy complete; nonblocking style warnings remain. Five mocked installer scenarios pass. The declared Rust 1.89 minimum passes a locked Windows binary compile check with that toolchain.

Eight headless About fixtures cover dark/light at 100/125/150/200% scaling. Selected dark/light renders were visually inspected. The guarded private-desktop executable rejects the normal desktop before creating UI resources. It then creates, presents and closes home and About windows for 60 cycles with both themes, confirms About dismissal and checks the input desktop is unchanged. This bounded exercise is presentation/lifetime evidence, not a full event-loop leak soak, actual tray-click acceptance or native visual approval. It performs no capture, clipboard, hotkey, tray, notification or upload operations.

Sandbox preparation tests check disabled host sharing/networking, exactly two narrowly scoped mappings, read-only input, XML escaping, preservation of existing evidence, script parsing and the guest preflight's refusal to run on the host. Windows PowerShell compatibility is checked separately. Preparation does not launch or enable Sandbox. The guest preflight itself has not run because no guest exists yet.

The remapped shipping executable/package passes icon-resource, public-identity, safe CLI, archive hash and quiet installed-file/shortcut verification. Existing application processes are preserved; no application is opened on the working desktop. Quit UL and reopen it when convenient to load 0.3.13.

## Publication status

The host has Windows 11 Pro, an active hypervisor and only the working console session. Sandbox executables, the Hyper-V management namespace and a configured remote Windows testing capability are unavailable. Administrator access is not present. The user chose to enable Sandbox later; no elevation UI, Windows feature change, restart or guest launch was attempted.

The [Sandbox kit](SANDBOX-TESTING.md) contains offline preflight and a native acceptance checklist. Real capture/paste, global hotkeys, timer interaction, Save/OCR/pin, provider uploads, genuine native README media, physical mixed-monitor DPI, memory soak and complete input latency remain pending. A private Win32 desktop does not replace an isolated VM/session for these workflows. The [0.3.12 audit](AUDIT-0.3.12.md) remains applicable. No Git commit, push or publication was performed.
