# Coordinated reliability and performance pass

Date: 2026-10-12. Source candidate; not installed or published.

## Scope and coordination

Ten requested Astra source-review streams completed first, followed by twenty additional bounded review streams when requested. The later five-agent cap was respected; delegation then stopped at the request to work inline. Changes from the parallel speed-review chat and the existing local baseline were preserved. No new end-user features or Cargo dependencies were added; OCR uses one additional feature of the already installed Windows bindings.

## Implemented fixes

- Copy and Save keep the editable capture on failure; explicit writes are tracked during graceful exit. Copy and cached-link reuse each allow only one pending worker. A cached link no longer holds the source image buffer while copying.
- Clipboard tickets are reserved before asynchronous history/pin decoding and before pixel capture after selection acceptance. Older work cannot overwrite a newer queued copy.
- Chosen-path saves reserve unique sibling staging files, preserve unrelated .part files, and publish only complete encoded images. Malformed image/crop inputs are checked before allocation or offset arithmetic.
- History serializes duplicate detection with publication, resets state after clear/direct saves, checks the saved path, orders new captures through clock rollback, excludes directories, and keeps Clear All available when images cannot decode. Clear covers both recognized local and legacy files and reports deletion failures. Prune failures are logged without misreporting the successful new save as failed.
- Whole-pixel panning reuses local sampled/composited layers. Pixelation caches use a 1 MiB budget and 256-entry bound; live drawing buffers are released on direct rendering and when fully clipped. Fractional image bounds use pixel centers. Pixelation and imported transparent-image previews preserve premultiplied alpha.
- Drag/resize transactions are committed or cancelled before deletion and undo can change indices. Text Enter uses the common commit path; unchanged re-edit preserves Redo, and active text sizing remains available. Text and step-marker resizing preserve the opposite anchor and supported sizes. Resize handles share geometry with the resize function.
- Timers use absolute deadlines and reject overlapping delayed requests through the capture grace period. Failed preview allocation no longer forces continuous allocation retries.
- OCR remains explicitly on demand, with one pending request before image allocation, bounded child output/runtime, cancellation, child-process job ownership and cleanup on graceful exit. An old capture completion still releases its operation gate. Application exit cancels and joins OCR; clipboard publication checks shutdown after acquiring the writer lock.
- HTTP replies must be 2xx, bounded and contain approved HTTPS image links. Legacy Imgur replies also require a successful JSON envelope and cannot redirect.
- Malformed settings are preserved before replacement; a failed recovery copy blocks saving. Installer updates preserve existing opt-in startup and owned Start replacement shortcuts; malformed, empty and custom paths are treated as unowned.

## Verification

The tested changes are integrated into the working source tree. The installed app is unchanged. Test data is synthetic; filesystem tests use isolated temporary directories, and installer tests mock Appx, startup registry and shortcut COM operations.

- Locked, offline all-target test run: **293 passed, zero failed, one intentionally ignored benchmark**. The normal parallel runner was used; this run emitted no compiler warnings.
- The Windows application passes locked/offline `cargo +1.89.0 check --bins` with its declared minimum Rust version, using a separate build target.
- Installer regressions passed in both PowerShell 7 and Windows PowerShell 5.1, including owned startup migration, replacement shortcut preservation and malformed/unowned paths. These were mocks, not changes to the user's Windows integration.
- Clippy completed without errors. Existing style warnings and warnings from the frozen benchmark reference remain; this is not a warning-free Clippy result.
- Source diff whitespace checks and public-identity scans passed. Thirty scoped files were integrated with pre-write hashes, backups and byte-for-byte checks against the tested candidate. Other contributors' changes were preserved.
- A separately built, remapped release binary is **9,921,024 bytes (9.46 MiB)**. Its CLI version path returned `snipping-tool-ul 0.3.13`; binary identity scans passed. SHA-256: `36a326a48ff92d6b091466db55ec4425143a961c8e028d4c34265390650ebfa7`. This remains an unreleased candidate at the existing version.
- Twenty-four source-rendered editing previews were generated without windows: dark/light themes, pencil/text/pixelate/highlight, and normal/minimum/150% layouts. Four representative images were visually inspected, covering both themes and all three layouts. Text descenders, selection boxes, tool labels and toolbar spacing appeared intact. These are synthetic renderer outputs, not native workflow screenshots or proof of DWM presentation.

### Renderer measurements

The release-mode CPU benchmark compares the current renderer with its frozen pre-fix implementation. Both use the same current annotation and text modules. Each sample creates a renderer and frame buffer, then renders 21 frames of a synthetic 1280 x 720 image into a 1920 x 1080 frame. Results are medians of nine samples after two warmups per implementation; old and new are measured sequentially on the same host.

| Scenario | Before | After | Interpretation |
| --- | ---: | ---: | --- |
| Whole-pixel pan with 12 arrows, 21 frames | 173.922 ms | 11.398 ms | 15.26 times faster in this fixture; resampling and composition each fall from 21 to 1 |
| Zoom with 12 pixelation regions, 21 frames | 250.693 ms | 258.704 ms | About 3.2% slower in this run; no zoom speedup established |

The pixelation cache rebuild count fell from 252 to 12 in the zoom fixture, but that did not improve total frame time. Alpha-correct rendering and remaining resampling/composition costs are included; the cause of the timing difference was not isolated. These numbers do not establish interactive latency, a universal speedup, memory peaks or superiority to Windows Snipping Tool.

The benchmark is reproducible with `cargo run --release --locked --offline --example astra_cache_bench`. It uses authored pixels only and creates no windows.

## Remaining acceptance and limits

- No working-desktop GUI, native capture/paste, global hotkey, mixed-monitor, native Save dialog or live upload test was run in this pass. These remain an isolated Windows acceptance gate. Native working screenshots/GIFs remain a publication gate.
- CPU-only renderer measurements are not end-to-end capture latency or a comparison against Windows Snipping Tool. OCR active CPU/memory measurement is owned by the separate OCR measurement task; synthetic lifecycle tests do not establish its resource peaks.
- Graceful exit waits at most 15 seconds for tracked file writes. OCR PNG encoding is not interruptible mid-encode; joining it may take longer than child cancellation. Abrupt termination may leave an owned temporary PNG; Windows job assignment also follows process spawn rather than being atomic with it.
- Installer rechecks startup before updating, but an unrelated process can still change that value between the final read and write. Strict cross-process transactional ownership is not implemented.
- Pruning failures are logged; an inline nonfatal retention warning remains a possible UI improvement. Very low-alpha pixelation can differ by a byte because averaged color uses truncation. No claim of exhaustive bug freedom is made.
