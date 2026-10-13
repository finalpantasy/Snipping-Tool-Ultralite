# Frozen capture

Implemented in source on 2026-10-13. The installed application was not updated in this pass.

## Default behavior

Starting New or using a capture hotkey hides UL's previous windows, waits for their removal to be presented, and takes one snapshot of the physical virtual desktop. Region and window selection display those same frozen pixels. The resulting snip is cropped from that snapshot, with no second screen read after dragging. Full-screen mode retains the snapshot directly.

For delayed capture, the snapshot is taken after the countdown, just before selection appears. A video continues playing underneath, but the displayed selection and copied pixels keep the frame present at snapshot time. Popups appearing later are absent from the snip. Existing layered windows are included using the capture raster-operation flag described in Microsoft's [BitBlt documentation](https://learn.microsoft.com/en-us/windows/win32/api/wingdi/nf-wingdi-bitblt).

The feature freezes the selection image, not the underlying applications. No continuous capture or background recording is added. Frozen selection is the default; no live-selection setting was added.

## Footprint

Selection temporarily owns one full-desktop pixel buffer. An exclusive guard converts it in place to Windows' native BGRA layout for selection, then restores RGBA on exit, including errors and unwinding. GDI uses a top-down uncompressed bitmap, following Microsoft's [BITMAPINFOHEADER documentation](https://learn.microsoft.com/en-us/windows/win32/api/wingdi/ns-wingdi-bitmapinfoheader). No second full-desktop CPU copy is added. A 1920 x 1080 buffer is 7.91 MiB; 3840 x 2160 is 31.64 MiB. Multiple monitors use their combined bounding rectangle. These figures describe the source pixel buffer, not total application or Windows compositor memory.

Cropping briefly allocates the selected pixels alongside that buffer, then releases the full desktop before clipboard publication and preview creation. Cancellation also releases the snapshot. Full-screen selection moves the existing buffer without copying it. Allocation and display failures report errors rather than silently taking a later live image. A display-layout change cancels the current selection.

## Verification

- After profiling and optimizing the painter, the locked, offline all-target test run passed **301 tests, zero failed, two ignored manual tests**, with no compiler warnings. Examples were compiled as test harnesses; their application entry points were not launched.
- Six new generated-pixel regressions cover changing live content, negative monitor origins, clipping, exact one-pixel output, full-buffer reuse, discarded desktop capacity, malformed inputs and oversized dimensions.
- A native GDI memory-DC test draws authored red/green/blue/mixed pixels through the production snapshot painter and verifies channel order and top-down rows. It creates no windows and reads no screen pixels. The format guard also has error/unwind restoration and buffer-reuse coverage.
- Scoped formatting and source diff checks passed. Earlier native OCR and other contributors' changes were preserved.
- No desktop capture, active GUI, clipboard, hotkey, timer, live upload, installation or publication was performed. Actual selection presentation, video-frame behavior, layered popups and mixed-DPI monitors still need the deferred isolated Windows acceptance session. DRM-protected or otherwise excluded content remains subject to the existing Windows capture backend's restrictions.

Evidence is retained in the local `audit/frozen-capture-20261013` folder: logs, scoped patch, baseline copies and source hashes. This is a source verification pass, not a packaged release or installation.

The initial RGBA-bitfield painter proved slow in a memory-DC benchmark and was replaced with the native-format path. See [frozen capture performance](FROZEN-PERFORMANCE.md) for measured costs and limits.
