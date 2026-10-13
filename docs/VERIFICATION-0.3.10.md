# 0.3.10 zoom and capture workflow correction

Editor New now starts capture after its mouse release, instead of during its press. The region picker also ignores releases before a legitimate press. Headless regressions cover one-shot activation, release outside the button, cancellation and an initial stray picker release followed by a valid selection.

Space fits and centers the image while Select is active. It is guarded during text entry, menus, size editing and active drags; Ctrl+0 remains available. Production client renders were inspected at minimum dark size and 150% light DPI, including the centered Select shortcut hint. Those renders use authored pixels, not desktop captures or native working-session screenshots.

Zoom sampling precomputes horizontal bilinear coordinates once per frame. Pixelation caches original block averages independently of zoom and pan, limited to eight grids and 1 MiB of color data. Oversized grids skip that allocation and calculate only visible samples. Pixelation draws solid block interiors directly into the viewport, avoiding millions of antialiased rectangles. No full-resolution flattened preview is retained; capture pixels and export rendering are unchanged.

## CPU rendering measurements

The release-mode production renderer used an authored 3840 x 2160 image, a 1410 x 918 viewport at 150% DPI, and nine successive pointer-centered zoom frames per case. Median times on this machine:

| Case | Before | After | Retained renderer bytes after |
| --- | ---: | ---: | ---: |
| Image only | 21.965 ms | 8.521 ms | 5,382,080 |
| 24 arrows and a 3,000-point pencil mark | 30.111 ms | 12.253 ms | 5,382,080 |
| Full-image pixelation, 12 px blocks | 62.500 ms | 8.929 ms | 5,554,880 |
| Full-image pixelation, 2 px blocks | 603.974 ms | 12.044 ms | 5,382,080 |

These measure CPU preview rasterization, excluding interface chrome, native presentation, input delivery, capture and clipboard work. They are not end-to-end app latency or total RAM measurements. Machine load can affect timings. The standalone headless example is `examples/zoom_bench.rs`; local before/after audit logs retain individual summaries.

## Verification and remaining gates

213 locked all-target tests pass. New coverage checks exact pixel equality with the previous bilinear sampler across DPI and zoom, original pixelation block means including partial edge blocks, cache reuse/invalidation and eviction limits, oversized-grid behavior, New activation and Select-only recentering. Formatting, Clippy without blocking errors (style warnings remain), whitespace and five mocked installer scenarios pass.

The remapped shipping build is verified through source/executable privacy scans, safe CLI paths, nine icon resources and three embedded pixel buffers, package entry hashes, and installed executable metadata/hash and Start shortcut/icon. Existing app processes are preserved; installation does not launch the app. No active-desktop windows, captures, clipboard writes, hotkey registrations or live uploads were used for testing.

Actual native capture/paste and the complete New/picker/input workflow remain isolated Windows acceptance gates. Prior private synthetic presenter evidence is described in [0.3.8 verification](VERIFICATION-0.3.8.md); it was not repeated for these renderer/input changes. Native appearance, mixed-monitor behavior, live image hosts, full-app performance and genuine working README media remain unverified. No commit, push, release or GitHub publication was performed for this update.
