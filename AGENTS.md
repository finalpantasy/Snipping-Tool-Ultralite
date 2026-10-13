# Snipping Tool UL

Ultralight in performance, not features. Keep capture, clipboard, and editor work independent of the launcher. No webview or permanent render loop.

Tests must run headlessly. Do not launch tray apps, pickers, previews, benchmark fixtures, or other windows on the user's active desktop. Do not capture the working desktop as test data. Native GUI testing needs a genuinely isolated VM/session. Record gaps until isolation is available.

The user's explicit permission for hidden desktop testing permits a narrowly scoped synthetic presentation probe on a private Win32 desktop. Its executable must refuse the normal desktop before creating an event loop; its harness must never switch the input desktop. No capture, clipboard, hotkey registration, tray, notification or upload operations are allowed in that probe. This is not a substitute for native visual or end-to-end workflow acceptance.

Keep personal names and machine-specific paths out of source, documentation, packages, and embedded build paths. Preserve third-party copyright and license notices. No upstream Git history or remote is part of this standalone repository.

No installation, startup registration, public upload, commit, push, publication, or destructive changes without explicit authorization.
