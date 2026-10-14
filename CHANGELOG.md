# Changelog

## 0.3.25 preview

- About now has a **Support on Ko-fi** button that opens ko-fi.com/finalpantasy in your browser.

## 0.3.24 preview

- Weekly update check, on by default: at most once every 7 days, at start, the app asks GitHub whether a newer version exists and shows an **Update available** link in Home and the tray menu. It never downloads or installs anything. Turn it off in **Settings > General > Check for updates weekly**.
- Fixed a possible crash when the editor was minimized from the keyboard with the annotation menu open.
- Fixed right-click during a text edit selecting, deleting or resizing the wrong annotation.
- Region drags that reach the right or bottom screen edge now include the last pixel column or row.
- The shortcut recorder accepts Ctrl+Shift+digit and digits on AZERTY keyboards.
- Switching tools mid-drag no longer leaves a ghost shape; the eyedropper no longer picks colors from outside the image; a failed history copy now says why.
- The editor holds about 5 MB less memory at the default size (33 MB less at 4K), and Copy, Save, Upload and Pin without annotations use half the peak memory.
- Copying an image prepares it fully before opening the clipboard, so other apps never see it locked or empty mid-copy. A 4K copy is about 20% faster to prepare and uses about 33 MB less temporary memory.
- Alignment pass: toolbar labels share one baseline, dropdown, gear, Undo/Redo, zoom, Settings title, host menu, upload notice and History grid are centred. See [the pass notes](docs/POLISH-PASS-2026-10-14.md).

## 0.3.23 preview

- Fixed an older instance, still running after an upgrade, reclaiming Start on login when Settings were saved. Startup now keeps a registration that points at a newer installed version.
- Fixed region snips growing past the drawn area when the mouse kept moving after release. The picker now uses each mouse message's own position instead of the live cursor; in a release-then-flick test, captures went from 1 of 10 correct to 15 of 15.

## 0.3.22 preview

- The canvas grows with annotations beyond the image and shrinks when they are moved, resized or deleted. Undo and redo restore the matching canvas; original image pixels stay intact.
- Copy, Save, Upload and Pin include the expanded canvas. The size label follows the live canvas, with white margins around overflow.
- Fixed fractional pixelation and spotlight boundaries changing source pixels after canvas expansion.
- Corrected expanded-pin positioning, including negative monitor coordinates. Oversized exports report an actionable error and keep the image for editing.
- Stabilized the OCR panic-cleanup test under parallel CI load without changing the application's timeout.
- 348 headless tests pass, including 20 canvas regressions and repeated rendering/effect stress cases. Native interaction acceptance remains pending isolation.

## 0.3.21

- Route Escape to an open menu before dismissing upload feedback underneath it, in both home and editor.

## 0.3.20

- Acknowledge Upload before preparing the image, show progress in the button and place a larger result directly underneath it.
- Distinguish link copied, upload succeeded but link copy failed, cached-link reuse and upload failure. Feedback never enters screenshot exports.

## 0.3.19

- Use light mode for new or unspecified preferences while preserving saved dark-mode choices.
- Show labelled light-mode app renders on GitHub; remove the separate product page and video.

## 0.3.18

- Open the empty launcher as a compact capture toolbar with the registered hotkey visible underneath. Expand dropdowns while open, then collapse on dismissal; captured images still open the full editor.

## 0.3.17

- Bring a completed capture's preview to the foreground after its first frame is presented. Ordinary redraws do not request focus.

## 0.3.16

- Minimize and hide the canvas and UL dialogs before capture or countdown; retain the previous preview when a timer is cancelled.
- Suppress capture-window fade transitions and keep hidden controls cloaked against late redraws or reopen events during selection.
- Show the actual registered screenshot hotkey in the tray tooltip and capture menu, including fallback and unavailable states.

## 0.3.15 preview

- Keep one active History copy and only the newest pending request, instead of decoding obsolete images in parallel.
- Transfer RGBA image storage without cloning it; reject oversized History images before allocating full pixel buffers.
- Make clipboard admission cancellable and bound all Quit stages with one shared deadline.
- Retain native OCR image ownership and the single-flight lease until Windows reports a terminal state, including status-query failures.
- Publish real capture and annotation screenshots, a short native UI demo, and a portable Windows preview.

## 0.3.14 preview

- Prepare ordinary PNG History thumbnails a row at a time, cancel obsolete refresh work and keep only one active loader plus the newest pending request. Preserve full-resolution clipboard copies.
- Reduce repeated opaque zoom and pixelation rendering work without adding image buffers; include pending History copies in graceful shutdown.
- Integrate direct Windows OCR with supervised cancellation and timeout handling, avoiding the per-request PowerShell bridge.
- Include coordinated annotation, settings, history, upload and installer reliability fixes verified by the headless regression suite.

- Freeze the desktop when selection starts, then copy the selected pixels from that same snapshot. Region and window selection show a still image; full-screen capture reuses the snapshot. Timers freeze after the countdown.
- Draw the snapshot without another full-screen CPU pixel copy, release it after selection/cancellation, and report allocation/display failures. Include layered windows already visible at capture time.

## 0.3.13 preview

- Replace the tray About notification with an on-demand themed window containing the logo, version and license information. Reopening About focuses the existing window; Close, X and Escape dismiss only About.
- Add an offline Windows Sandbox preparation kit with an isolated results folder, read-only app input and host clipboard/device sharing disabled. It prepares files only; Sandbox enablement and native workflow acceptance remain pending.
- Verify the minimum Rust 1.89 toolchain and exercise home/About native presentation on a guarded private desktop, keeping the user's working desktop untouched.

## 0.3.12 preview

- Move Capture history to the top of Options and show Currently off/on. Load history thumbnails in the background and preserve PNG/JPEG/WebP formats with collision-safe, complete-file publication.
- Reuse unchanged annotated viewport pixels with bounded metadata; accelerate pixelation export with direct pixel fills and clip oversized regions to the source image.
- Keep Settings open with an explanation when saving fails. Preserve the working hotkey if a replacement fails, pending clipboard work when capture is cancelled, and the selected capture mode after a tray window capture.
- Schedule the earliest timer/render deadline so an idle branch cannot erase countdown ticks. Close startup registry handles on failures and clean partial OCR temporary files.
- Remove full-image copies from OCR/pin encoding and allow longer bounded upload writes on slow connections. Declare Rust 1.89 to match the resolved Windows dependency requirement.

## 0.3.11 preview

- Render Capture History text, borders and thumbnails at native display resolution instead of stretching a 600 x 400 image. Refresh on display scaling changes while retaining logical click targets.
- Explain when history is off versus enabled but empty. History remains optional, stores up to 20 images locally and is separate from Auto-save.
- Isolate history storage tests in a temporary directory so pruning never touches the user's saved images.

## 0.3.10 preview

- Start a new editor capture after releasing New, and ignore leftover mouse releases in the picker, so one click starts a new selection.
- Speed up edited zoom with precomputed bilinear coordinates, a 1 MiB pixelation color cache, and visible-pixel drawing instead of millions of block rectangles. Original capture/export pixels stay unchanged.
- Space recenters and fits the image only with Select active, outside text entry, menus and active drags. Show the shortcut in the Select hint.

## 0.3.9 preview

- Give arrows broader heads and clean pointed tips at every stroke width. Draw one silhouette so the shaft cannot protrude through the head or darken translucent joins.
- Keep short arrowheads within the arrow length, and include the full head in selection bounds and click targets.

## 0.3.8 preview

- Add Catbox, Freeimage.host and four temporary Litterbox expiry choices in a no-setup host picker, with public-upload disclosures and expiring link reuse.
- Pan the image by dragging with the middle mouse button; wheel scrolling remains pointer-centered zoom.
- Rename Move to Select, enlarge annotation click targets consistently across zoom/DPI, and center tool hints and toolbar labels.

- Space new upload attempts at least five seconds apart, including retries, without hourly or daily caps. Cached links remain instant; confirmation and cancellation do not consume the cooldown.

## 0.3.7 preview

- Use one embedded Open Sans typeface for interface and annotations, remove the font selector, and load glyphs lazily.
- Reduce runtime SVG features, idle countdown redraws and unnecessary capture-buffer copies.

- Use bold saturated annotation colors instead of pastel defaults, with matching editor and Settings swatches and accurate tooltips.
- Preserve custom hex colors, highlight transparency and the teal frosted interface.

## 0.3.6 preview

- Keep Options open while changing preferences and show On/Off switches.
- Replace upload-confirmation toasts with Upload publicly and Cancel controls.
- Reuse links for the eight most recent successful images and destinations during this session; changed pixels require another upload.
- Clarify the auto-save folder in General settings, honor custom destinations and create missing folders.
- Avoid overwriting captures with identical timestamps.
- Open the manual Save chooser before rendering annotations or encoding the image; save on a worker after choosing a filename.

## 0.3.5 preview

- Show Snipping Tool UL as the Windows process description in Task Manager.
- Add a separate Quit command to Options and the editor gear menu.
- Offer Keep in tray or Quit completely for the main close button in General settings.
- Clarify Options groups, toggle states and image host; preserve the frosted workspace.
- Update TLS, Windows notification and memory-map dependencies after advisory checks.
- Prepare publication checks and distinguish headless evidence from native acceptance gaps.

## 0.3.4 preview

- Keep the native caption and command band the same themed color with acrylic or fallback.
- Use teal charcoal in the default dark theme and pale teal neutrals in optional light mode.
- Remove the ivory tile from app, title and taskbar icons; keep the faceted teal bird.
- Use adaptive teal tray silhouettes, prepared offline with all app icon sizes.

## 0.3.3 preview

- Use the supplied teal bird/scissors app icon, preserving all nine Windows ICO sizes.
- Use the dedicated tray silhouette with a light/dark Windows taskbar tint chosen at startup.
- Embed prepared icon pixels to keep SVG parsing and PNG decoding out of startup.

## 0.3.2 preview

- Replace blue interface accents with teal inspired by the scissors color reference.
- Keep primary labels readable and adapt aqua selection/focus to each theme.
- Preserve frosted surfaces, annotation colors and the temporary shell icon.

## 0.3.1 preview

- Center icon/text button content together and enlarge toolbar labels.
- Clarify Zoom and hide empty confirmation icons.
- Add stronger selection and keyboard-focus outlines across themes.
- Redraw correctly when switching from keyboard focus back to the pointer.

## 0.3.0 preview

- Live width, text font/size, step size and pixelation controls.
- Rounded pencil curves, glyph-aware text bounds and centered button labels.
- Delete numbered steps with automatic renumbering and grouped undo.
- Pixelate/highlight guides, eyedropper feedback and clearer tool help.
- Cached viewport editing, input coalescing and original-resolution exports.
- Stronger light/dark frosted surfaces, translucent command bar and native acrylic caption integration.
- Standalone repository and pinned Windows CI prepared; publication awaits final identity.

## 0.2.2

- Physical-DPI settings rendering, clearer tools and cached assets.

## 0.2.1

- Opt-in current-user Snipping Tool replacement and Start integration.

## 0.2.0

- Frosted native home/editor, pointer-centered zoom and Catbox upload.
