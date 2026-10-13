# Reliability and rendering optimization pass

Completed inline on 2026-10-13. Source changes are verified; the installed app is unchanged.

## Findings and fixes

### History copy could be interrupted by Quit

History reserved its clipboard ticket before decoding, but its worker handle was detached. Graceful shutdown waited for tracked file writes and explicit editor/pin copies while ignoring a pending History copy.

A generated worker blocked on a channel reproduced the defect: shutdown returned before that accepted task finished. The clipboard spawn helper now uses the existing background-worker tracker, preserving ticket reservation before work starts and rejecting superseded writes. The same regression passes after the fix. No additional worker thread is introduced.

The shutdown wait retains its existing 15-second application deadline. This does not promise completion after forced termination or an operation that outlasts that deadline.

### Opaque screenshot sampling did unnecessary alpha work

The zoom sampler interpolated all four channels and checked whether premultiplication was needed for each source sample. Opaque screenshots now use a specialized row sampler that interpolates RGB and writes alpha 255 directly. Imported transparency uses the existing premultiplied blend order.

Existing bilinear pixel-oracle tests pass, with an added independently calculated opaque/translucent midpoint and transparent-edge test. Viewport geometry and export resolution are unchanged.

### Pixelation repainted repeated scanlines

Several output rows often map to the same source-block row. Fully covered interior pixels now copy the preceding identical output scanline in place. Partially covered edge pixels retain their row-specific blending. Cached and oversized fallback grids share this optimization.

The output matches the frozen pre-pass implementation byte for byte for transparent sources, fractional edges, negative origins, different scales and cache-budget fallback. This adds no image buffer or dependency.

## Paired CPU measurements

The benchmark compares the current renderer with a frozen copy from the start of this pass in the same release-optimized process. Both use the same current annotation, font and layout modules. Each sample creates a renderer/frame buffer and renders 21 frames of an authored 1280 x 720 image into a 1920 x 1080 frame. Medians use nine samples after two warmups per implementation. Implementations run sequentially, so competing host load can affect measurements.

| Scenario | Pre-pass median | Updated median | Reduction in measured time |
| --- | ---: | ---: | ---: |
| Whole-pixel pan, 12 arrows, 21 frames | 12.736 ms | 9.952 ms | 21.9% |
| Zoom, 12 pixelation regions, 21 frames | 378.663 ms | 271.978 ms | 28.2% |

Sampling/composition/grid counters are unchanged in the paired fixture, isolating faster work rather than a dropped frame or missing annotation. These are synthetic renderer measurements, not complete UI frame times, capture-to-paste latency or a comparison with Windows Snipping Tool.

Reproduce with `cargo run --release --locked --offline --example optimization_bench`. It creates no windows and reads no screen or user files. Earlier exploratory benchmark logs used an older reference; the paired fixture above uses this pass's actual starting renderer.

## Verification

- Locked/offline all-target suite: **304 passed, zero failed, two ignored manual tests**, with no compiler warnings.
- The shutdown regression failed before the fix and passes afterward. Clipboard-order tests continue to reject delayed older work.
- Bilinear output, live/committed layer reuse, annotation invalidation, pixelation cache bounds and fractional/translucent edge parity passed.
- Scoped formatting, diff and public-identity checks passed. Baseline copies and source hashes are retained locally in `audit/optimization-pass-20261013`.
- Thread-pattern review found History as the remaining explicit-copy producer using the detached helper. Pin/editor writes already use tracked workers; network uploads remain detached. Thumbnail preparation is cancellable background reading, and OCR retains its separately integrated lifecycle.
- Frozen capture and native OCR source changes were preserved. No installation, commit, push or publication occurred.

The UI source preview and final hash checks are recorded with the local audit evidence. Actual Windows capture/paste, global hotkeys, mixed-DPI presentation, active glass appearance and full-app latency/memory still require the deferred isolated Windows session. No working-desktop GUI, screen capture, clipboard, notification, tray or live upload operation was used in this pass.
