# 0.3.1 interface polish

The frosted material and capture/export behavior from 0.3.0 are preserved.

- Action buttons center the combined icon and text using physical glyph bounds.
- Bottom labels increase from 9.5 to 10.5 logical pixels, with room below selection outlines.
- Zoom replaces the ambiguous Reset label. Its tooltip explains fitting the image and Ctrl+0.
- Empty status messages draw no confirmation icon. Existing nonempty confirmations remain visible.
- Selected tools have a theme-aware outline. Keyboard focus outlines the whole control with a gap, including menus and toolbar labels.
- Returning to the mouse redraws keyboard focus even when the pointer stays on the same button.

175 Rust tests pass, including existing physical-DPI layout, reachability, premultiplied glass, original-resolution export and keyboard-navigation checks. The keyboard checks now cover returning to the pointer on the same target. Five mocked installer scenarios also pass. Headless renders cover both themes, default/minimum window sizes, 150 percent editor DPI, 100/150/200 percent settings DPI, and focused New/tool/Zoom controls.

Synthetic renders use actual client UI code over authored backdrop colors. They create no windows, capture no desktop, write no clipboard data and perform no uploads. No dependencies, per-frame blur or permanent render loop were added. Native active appearance and end-to-end acceptance remain separate; see [0.3.0 verification](VERIFICATION-0.3.0.md).

Portable packaging verifies safe CLI commands, source/executable identity scans, archive entries and hashes. Installed updates preserve existing app processes; Quit from the tray and reopen to load the new executable. GitHub publication and a final logo remain pending.
