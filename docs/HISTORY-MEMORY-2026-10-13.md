# History memory and repeated-use cleanup

Completed inline on 2026-10-13. Source changes only; the installed app is unchanged.

## Changes

History previously decoded each PNG into a full-resolution image before shrinking it to a thumbnail. A refresh started another thread even when the cancelled previous decoder was still running. Rapid refreshes could overlap large image allocations.

Ordinary non-interlaced 8-bit PNG captures now decode one row at a time into the existing box-filter thumbnail. The loader retains a row, small accumulators and the thumbnail rather than a full-resolution image. Indexed PNG transparency is expanded correctly. Pixel output matches the previous thumbnail implementation across RGB, RGBA, grayscale, grayscale-alpha, indexed transparency, different aspect ratios and downscale factors.

Refreshes share a single preparation queue. A newer request cancels the active request and replaces the pending request, so only one decoder runs at a time. PNG cancellation is checked between source rows. Replaced callbacks release their payloads; the worker exits when idle. Weak window references prevent preparation from retaining a closed History window. Completion disconnects the result channel before waking the UI, including cancellation and panic cleanup.

History copies still decode the original image for full-resolution clipboard output. Saving captures, the history storage limit and user preferences are unchanged. No webview, permanent thread, render loop or new codec package version was added. The existing transitive PNG codec is now also a direct dependency for its row API.

## Headless measurements

Release builds measured generated RGBA PNGs in fresh processes, three samples per size and implementation. Input fixtures already existed; image generation is outside the measured interval. Both implementations produce 120 x 68 thumbnails. The legacy branch reproduces the previous full-decode preparation path.

| Input | Previous peak Rust heap | Updated peak Rust heap | Previous median decode | Updated median decode |
| --- | ---: | ---: | ---: | ---: |
| 3840 x 2160 | 31.92 MiB | 0.34 MiB | 43.065 ms | 38.910 ms |
| 7680 x 4320 | 126.84 MiB | 0.37 MiB | 203.272 ms | 158.705 ms |

Peak reductions are 98.9% and 99.7%, respectively. Single-load allocations return to the pre-load baseline after dropping the result. These are requested Rust heap bytes, excluding allocator overhead, native allocations and OS memory; they are not total process RAM. Sequential measurements can be affected by host load and file caching. This is not an end-to-end UI latency measurement or a Windows Snipping Tool comparison.

Reproduce using `cargo build --release --locked --offline --example history_memory_bench`, then run the example with `3840 legacy`, `3840 stream`, `7680 legacy`, `7680 stream` and `3840 soak` from a disposable audit directory. The example creates only authored fixtures and uses no windows, desktop capture or clipboard.

## Bounds and compatibility

- PNG metadata/decoder budget: 16 MiB. Source dimensions: at most 65,536 per axis and 134,217,728 pixels.
- Encoded file size: at most 256 MiB. Thumbnail targets are capped at 1024 pixels per axis for extreme scale factors.
- Interlaced, animated, 16-bit or upscaled PNGs, JPEG and WebP retain the existing image-library thumbnail path. It limits decoded output to 128 MiB and applies codec allocation limits where supported; this is not a hard total-memory cap.
- Fallback codecs cannot be interrupted during decoding. Cancellation is checked before and after decoding, and the shared queue prevents overlapping fallback decoders.
- Corrupt or oversized images skip their preview. Stored-entry counts remain separate from successfully decoded thumbnails, so Clear All remains available.

## Verification and remaining boundary

The regression for overlapping refreshes failed before queue replacement and passes afterward. Eight focused tests cover byte parity, indexed transparency, cancellation before draining PNG input, bounded malformed inputs, compatible JPEG/small/16-bit PNG fallback, 400 replaced payloads being released, coalescing and recovery after a panicking callback.

A synthetic repeated-use run issued 500 History reload/cancel requests plus 200 crop/render/drop cycles. Process handle count remained 77. Retained Rust heap was 256 bytes from the benchmark's growing results vector, with no image-buffer accumulation; peak was 1,709,056 bytes. This exercises production decoding, crop and preview-render functions with generated pixels. It does not open native windows.

The locked/offline all-target suite passed **312 tests, zero failures, three ignored manual tests**, with no compiler warnings. The ignored resource soak was run separately: 256 production memory-DIB draw/drop cycles returned to zero GDI objects and 119 process handles. A live probe DC increased the GDI counter before cleanup, validating that the zero baseline was meaningful. These tests do not create HWNDs or read the desktop.

The generated 200% thumbnail was visually inspected. Formatting, whitespace and scoped public-identity checks passed. The lockfile adds only a root dependency edge to the already resolved PNG codec. Baselines, logs and final source hashes are retained locally in `audit/history-cleanup-20261013`.

Actual History window open/close, mixed-DPI presentation, full-app long-session memory, capture/paste and native interactions remain deferred until an isolated Windows session is available. No working-desktop GUI, capture, clipboard, notifications, tray, uploads, installation or publication occurred.
