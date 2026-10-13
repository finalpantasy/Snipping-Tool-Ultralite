# 0.3.19 light default and GitHub presentation

New or unspecified configurations use light mode. Explicit saved dark choices still work, including mixed-case values. Invalid theme values resolve to light; the process palette also starts light before config is applied. The earlier compact launcher, capture hide guards and completed-preview foreground request are included.

GitHub is the project presentation. The separate Pages site, standalone video and current dark gallery were withdrawn. The user approved light-mode app-code renders instead of fresh native screenshots. They are clearly labelled, use authored sample pixels and include no private desktop content. See [media provenance](MEDIA.md).

## Verification

- 323 locked offline all-target tests passed; 0 failed, 3 manual tests ignored.
- Existing theme regressions now cover missing preferences, explicit dark choices, case handling and invalid values. Compact-launch/menu and prior capture/editor regressions remain covered.
- Light editor and compact home app-code renders inspected; format/diff checks and mocked current-user integration passed.
- Remapped production build, source/package/commit identity scans, safe version/help, archive entry hashes and installed executable/metadata/notices/shortcut read-back.

The installed user's theme is set to light as requested; other saved preferences are preserved. No application or test window is launched on the working desktop. Native resize/foreground/capture exclusion, mixed-DPI, paste, timers, live host uploads and full interactive resource acceptance remain pending isolation. This is a preview, without a verified speed comparison against Windows Snipping Tool.
