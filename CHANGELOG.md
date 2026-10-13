# Changelog

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
