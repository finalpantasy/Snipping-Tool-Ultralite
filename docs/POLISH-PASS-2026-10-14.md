# Bug, memory and alignment pass, 2026-10-14

All checks ran headlessly. No window was opened, no desktop was captured and the clipboard was not touched.

## Bugs fixed

Each fix has a test that fails without it, except the history notice, which only adds a notification.

| Bug | What the user saw | Fix | Test |
| --- | --- | --- | --- |
| Editor crash | Right-click an annotation, then minimize the editor from the keyboard: the app could crash and lose the capture. | Menu position limits stay ordered for a 0x0 window. | `editing_tests::annotation_menu_survives_a_minimized_zero_size_editor` |
| Wrong annotation after a text edit | Right-clicking while re-editing text could select, delete or resize a different annotation, and undo could swap layer order. | Pending text is committed before the right-click picks a target, and before Delete. | `text_commit_tests::right_click_during_text_reedit_selects_the_clicked_annotation` |
| Region drag one pixel short at the screen edge | Dragging to the right or bottom edge always left out the last column or row; a corner-to-corner drag gave 1919 x 1079 on a 1920 x 1080 screen. | Reaching the last pixel now includes it. Drags inside the screen are unchanged. | `region_picker::drag_to_the_last_screen_pixel_reaches_the_edge` |
| Shortcut recorder ignored Ctrl+Shift+digit | Ctrl+Shift+1 arrives as "!", so it was refused silently; on AZERTY keyboards no digit worked. | The recorder reads the physical key, as the global hotkey does. | `hotkey_tests::recorder_uses_physical_keys_so_shifted_digits_and_azerty_work` |
| Ghost shape after switching tools mid-drag | Pressing a tool key while still dragging left the old tool half-drawn; its shape reappeared later and changed the size label. | The unfinished stroke is dropped on a tool switch. | `editing_tests::switching_tools_mid_drag_leaves_no_stale_shape` |
| Eyedropper outside the image | Left of or above the image it picked the first column or row; past the right or bottom edge it kept the last color. | Outside the image there is no color. | `editing_tests::eyedropper_has_no_color_outside_the_image` |
| History copy failed silently | Clicking a very large or unreadable history item did nothing visible. | A notification explains why nothing was copied. | Notification only, not headless-testable |

Not fixed here: a pin moved between monitors with different scaling keeps its old size (needs a real window to test), and the suspicious items listed in the review notes.

## Memory and speed

Measured with a counting allocator in a headless probe, comparing the previous commit with this branch.

| Measurement | Before | After |
| --- | --- | --- |
| Editor chrome cache, 940 x 612 window at 150% | 5.18 MB | 0.14 MB |
| Rust heap while the editor is open, same window, 1080p capture | 16.07 MB | 11.39 MB |
| Chrome cache at a 4K-sized editor | 33.18 MB | about 0.4 MB |
| Copy, Save, Upload or Pin with no annotations, peak extra memory, 1080p | 16.59 MB | 8.29 MB |
| Same, 4K | 66.36 MB | 33.18 MB |
| Chrome replay per warm frame | about 0.20 ms | about 0.05 ms |

The chrome cache now stores each run of identical rows once and replays them byte for byte (`chrome_cache_tests`). Export skips a second full copy when there is nothing to draw.

About the 43 MB measured after a capture: that figure was taken while the editor was open. After the editor closes, the probe found only 0.71 MB of Rust memory left, all of it the font. The rest of the open-editor cost is the window frame, the image layers and an estimated 10 MB of Direct2D buffers that cannot be measured without a window. Reading private bytes after closing the editor would confirm this.

# Alignment

Small layout errors, one to eight pixels, measured in headless renders of the real drawing code and then fixed.

Pictures: [light](images/alignment-before-after-light.png) and [dark](images/alignment-before-after-dark.png), rendered at 150%.

## How it was measured

`cargo run --example alignment_probe -- <folder>` renders the home and editor toolbars, every launcher menu, the upload notice, the editor dock, Settings and About in light and dark at 100% and 150%. For each control it finds the drawn pixels and reports how far the icon, the label's capital letters and the whole content block sit from the control's centre, plus each label's baseline row. The same probe ran against the previous commit for the "before" numbers.

## What changed

Offsets are physical pixels at 100% scaling unless noted. Half a pixel is the rounding floor.

| Place | Before | After | Cause |
| --- | --- | --- | --- |
| Toolbar labels (New, Rectangle, Delay, Copy, Save, Upload, Options) | Baselines 2 px apart | One baseline | Each label was centred on its own ink, so labels with g, y or p rose 2 px. Labels now centre their capital letters. |
| Rectangle and Delay dropdowns | Content 7 to 8 px right of centre | Within 1.5 px | Icon and label were centred over the whole button, then the chevron was added on top. Icon, label and chevron are now one centred block. |
| Editor toolbar gear | 2 px right of its button | Centred | Icon and hover box were offset from the click target. |
| Undo and Redo icons | 1.5 px left of their buttons, 1.5 px low inside the icon | Centred | Icon drawings ran from 4 to 23 of 24 units. Redrawn on 4 to 20, and placed on the target centre. |
| Zoom percentage | 2 px below the icon line | On the line | Text was placed by its top edge. |
| Tools icon and label | 1 px right of the button | Centred | Off-by-one x offset. |
| Settings title | 2 to 2.4 px below the gear | Level | Placed by top edge. |
| Image host menu icons | 4 px high (6 px at 150%) | Centred in the 40 px rows | A 32 px row offset was reused. The check mark had the same issue. |
| Upload notice close button | 3 px low against the heading | Level | The close mark was a text glyph. It is now an icon centred on its target, which shares the heading's middle line. |
| Check icon | 1 unit high in its box | Centred | Drawing offset. Affects Settings check boxes and menus. |
| History grid | 16 px left margin, 68 px right | 42 px both sides | Grid started at the page padding. Header title and Clear All share the grid edges, and the title is level with Clear All (it was 3.4 px low). Clear All is rounded like other buttons. |
| Editor size controls | Gaps 6, 7, 6 and 4 px | 6, 6, 7 and 6 px | Plus button sat closer to the slider than other controls. |
| Countdown digits | About 2 px low | Centred on their ink | Centred by line height, not by the digits. |
| Overlay tooltip, selection size and eyedropper hex labels | 2 to 3 px low in their pills | Centred | Same top-edge placement. |

All toolbar baselines now match at every scale tested (100%, 125%, 150%, 200%). `tests/alignment_tests.rs` keeps that true and checks the icons used in buttons stay centred in their boxes.

## Not changed

- The editor toolbar keeps its right-hand group close to the window edge. Moving it in to match the 18 px left margin would collide with the left group at the 660 px minimum width.
- Filled (primary) buttons show a large "caps" offset in the probe output both before and after. Their measured baselines equal those of the neighbouring buttons, so this is a quirk of the probe's first-letter detection on filled buttons, not a drawing error.
- The countdown and overlay labels were checked by arithmetic, not by the probe, because their renderers write straight to a window surface.
