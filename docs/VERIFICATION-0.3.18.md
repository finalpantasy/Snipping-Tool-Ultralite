# 0.3.18 compact empty launcher

The no-image launcher previously retained a 660 × 372 logical-pixel blank workspace. It now opens at 660 × 112 with capture controls and the actual registered shortcut visible below them. Menus and optional connection controls temporarily expand to the prior height and collapse on dismissal. Completed screenshots still use the full editor; the capture hiding and post-capture foreground behavior are unchanged.

The launcher allocates and presents only its current physical-size surface. Pointer mapping uses the same width-based DPI scale as the renderer, including a pending native height change. No webview, dependency, animation, resident worker, timer, or render loop was added.

## Verification

- Locked offline all-target tests, including compact launch/menu expansion and dismissal, menu target bounds, New and keyboard navigation.
- Actual app-code dark/light compact and expanded menu renders inspected headlessly, including a 200% focused view.
- Formatting, whitespace, mocked current-user integration and remapped production build.
- Package file hashes, source and executable identity scans, safe version/help commands and installed executable/metadata/Start target read-back.

No app, picker, clipboard, tray, screenshot capture or foreground operation is launched on the working desktop. Native window resizing, mixed-monitor DPI and complete capture workflows remain an isolated-session acceptance gap. This update is locally installed; no new GitHub release is published.
