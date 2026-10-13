# 0.3.8 no-setup sharing and viewport controls

## Implemented

Options > Image host selects Catbox, Freeimage.host or Litterbox with 1-hour, 12-hour, 1-day or 3-day expiration. No account or user-supplied API key is requested. Freeimage.host uses its documented shared public application key; its menu and confirmation disclose public discoverability. Uploads return strictly validated direct HTTPS PNG links and never rotate hosts automatically. The five-second cooldown spans destinations; expired temporary links are no longer reused.

Middle-mouse dragging pans only the image viewport. Wheel scrolling still zooms around the pointer; Ctrl+0 resets the view. Select (V) edits annotations, with larger button targets and a nine-logical-pixel selection tolerance that stays consistent across DPI and zoom. Context hints, size controls and footer labels are centered using glyph bounds. Dark-first teal frost and transparent bird icons remain.

The combined source also includes the coordinating font and Settings work: one embedded Open Sans typeface and lazy glyph loading, without a font picker. This checkpoint does not independently reproduce earlier font-memory measurements.

## Evidence and limits

- 201 locked all-target tests pass, including eight new no-setup-host and viewport tests. Exact offline multipart bodies, provider response validation, config migration, host menus, expiry boundaries, global cooldown and zoom/DPI pan/selection transforms are covered.
- Rust formatting passes. Clippy completes without blocking errors; existing style warnings remain. Treating every Clippy warning as an error does not pass, and unrelated style changes were not made.
- Five mocked current-user installer scenarios pass; those use mocked Appx operations.
- Actual-source headless renders were inspected in dark and light themes at default/minimum sizes and 150% DPI, including Select, provider menus and public-upload prompts. Render content and backdrops are synthetic, not working native screenshots.
- The portable build script now builds shipping binaries separately from optional examples so example-only SVG/font dependency features do not enter the shipping build. Dependency notices include 218 resolved registry entries, including build/dev dependencies; this count is not the runtime dependency count.

Fresh portable-build, resource, CLI, package/hash, private-desktop presentation and installed-update results are recorded in the local audit. Installation preserves existing app processes and does not launch the update.

No active-desktop test windows, captures, clipboard writes, hotkey/tray registration, notifications or HTTP uploads were used. Live provider responses, actual middle-mouse native events, capture/paste, mixed monitors, native frosted appearance, timers/hotkeys and end-to-end latency remain isolated Windows acceptance tasks. Private synthetic presentation checks do not establish those behaviors. Genuine native working README media is still required before GitHub publication. See [publication checklist](PUBLICATION.md) and [host policies](UPLOAD-HOSTS.md).
