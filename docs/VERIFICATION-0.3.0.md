# 0.3.0 preview verification

## Editing and interface

175 Rust tests pass. Five sandbox installer scenarios pass with mocked Appx operations: preview, alongside installation, takeover/idempotence, shortcut conflict and removal failure. No real uninstall occurs in these tests.

Headless fixtures render the actual editor at default and minimum sizes, including 150 percent DPI, in light and dark themes. Settings renders all four tabs at 100/150/200 percent. Synthetic renders are inspected separately from native acceptance.

Live width, text size/font and step size controls preserve original-resolution export. Pencil input accepts half-pixel samples and renders rounded curves. Text bounds include glyph descenders. Preview guides show pixelate/highlight regions and selection handles, without appearing in exports. Deleting a step renumbers later steps as one undoable operation. The logo is undecided after rejected proposals; existing shell icon assets are temporary.

## Stronger frost refinement

The shared body tint is cool graphite in dark mode and pale gray-blue in light mode. The opaque command band is replaced with a translucent material, and the image well lets more of the backdrop show through. Screenshot pixels remain opaque and original-resolution export stays independent of the UI.

Secondary body labels measure 4.79:1 contrast over the worst-case white backdrop in dark mode and 4.80:1 over black in light mode. The headless material_preview helper renders actual home/editor/settings clients over authored backdrop colors; it does not capture the desktop or reproduce native DWM composition.

When native acrylic is available, the caption's explicit solid color is reset through the documented [Windows DWM attributes](https://learn.microsoft.com/en-us/windows/win32/api/dwmapi/ne-dwmapi-dwmwindowattribute). Solid fallback restores the themed caption. A private-desktop probe completed ten alternating dark/light presentation cycles with no input desktop switch. Active caption/material appearance remains a native visual acceptance gap. No dependencies, per-frame software blur or continuous render loop were added.

## Performance

Same-run release CPU measurements at a 1410 by 918 viewport, 150 percent DPI, compare the previous full-source frame pipeline with the cached viewport pipeline. Median of 21 warm frames, synthetic capture without annotations. These exclude capture, clipboard, native presentation and process overhead.

| Capture | Previous CPU frame | Cached viewport CPU frame | Previous retained pixel bytes | Cached retained pixel bytes |
| --- | ---: | ---: | ---: | ---: |
| 960 by 540 | 6.333 ms | 3.739 ms | 9,324,720 | 13,825,008 |
| 1920 by 1080 | 7.346 ms | 3.768 ms | 21,766,320 | 20,045,808 |
| 3840 by 2160 | 9.477 ms | 3.829 ms | 71,532,720 | 44,929,008 |

The cached path resamples once across the warm frames. Its buffers hold the immutable source capture, visible-image cache and frame; they are not total process RAM. Zoomed small captures can use more cache memory. Annotations, pixelation and zoom/pan invalidation add work. Event-driven input rendering is coalesced to an 8 ms interval; this is a scheduling limit, not measured frame rate.

Home warm CPU rendering was 0.309 ms, with a 982,080-byte framebuffer; first rendering was 30.727 ms. Settings warm CPU rendering was 0.241/0.386/0.497 ms at 100/150/200 percent DPI. Timings vary with machine and load; these values were measured while the PC was in use.

## Native and release boundaries

A guarded private-desktop helper completed ten synthetic home presentation/theme/lifecycle cycles, with the input desktop unchanged. It performed zero capture, clipboard, hotkey, tray, notification or upload operations. Normal-desktop execution is refused before creating an event loop. This does not establish active glass appearance or full-product memory/latency.

Remaining isolated Windows acceptance: real capture/paste into receivers, hotkeys/timers, mixed DPI monitors, live uploads and Reddit/RES display, installer dialog appearance, native editing interaction and full-app performance. Do not run GUI fixtures on a working desktop.

Release packaging checks safe CLI commands, file hashes, ZIP entries, source/executable identity markers and embedded build paths. Public commits use the account handle and GitHub noreply identity. Windows CI uses pinned action revisions and runs formatting, Rust tests, mocked integration and portable release compilation. CI results are reported separately after publication.
