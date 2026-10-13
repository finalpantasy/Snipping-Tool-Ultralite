# Performance and correctness review, 2026-10-12

The current feature set is preserved, as requested. This pass adds no dependencies, polling, permanent workers, or new user controls. Four independent read-only reviews covered rendering, capture/resource lifetimes, background workflows, and the final patch. All source writes in this pass had one owner.

## Changes made

- **Pinned-image cleanup:** the window and background encoder share one temporary-file owner. Closing before the encoder creates the PNG, or while its file is open, now cleans the file after the encoder releases it. A per-process sequence also prevents same-millisecond pin filenames from colliding. Two deterministic, headless lifecycle tests cover these races.
- **Clipped text:** determine glyph visibility from metrics before allocating/rasterizing its mask. Fully clipped glyphs do no raster work; partially visible glyphs blend only intersecting rows and columns. Baseline comparisons cover all viewport edges, fractional positions, Unicode, transparency, and sizes from 8 to 256 pixels. Partially visible glyphs still allocate their full glyph mask.
- **Numbered steps:** measure digit advance widths with font metrics instead of rasterizing and discarding a first bitmap. Rendering remains byte-identical for one-, two-, three-, and ten-digit values at four sizes. The full marker fixture showed no measurable net speed gain; the redundant rasterizations and bitmap allocations are nevertheless removed.
- **Oversized pixelation fallback:** reuse one visible row of block colors when multiple output rows map to the same source blocks. Temporary scratch is bounded to three bytes per visible output column, or 3,840 bytes in the 1,280-pixel benchmark. The persistent full-grid cache keeps its existing 1 MiB budget. Cached and fallback output matches the previous renderer, including fractional coverage and clipped/partial blocks.
- **Memory reporting:** include the live annotation pixmap in `PreviewRenderer::retained_bytes()`. A test checks both its count during editing and release when the live mark ends.

## Measurements

Release builds, synthetic data, three warmups and the median of 31 samples. The previous functions were copied verbatim from the pre-change snapshot and compared in the same executable. These are isolated CPU timings, not whole-app input latency or process memory.

| CPU fixture | Previous | Updated | Result |
| --- | ---: | ---: | --- |
| Small visible text | 0.0101 ms | 0.0101 ms | Unchanged |
| Clipped text at 256 px | 0.5319 ms | 0.0248 ms | 21.45x faster |
| Fully offscreen text at 256 px | 0.5208 ms | 0.0008 ms | Rasterization eliminated |
| 24 numbered steps | 1.1746 ms | 1.1861 ms | No measurable net improvement |
| 4K pixelation, block 2, scale 1, 1280x720 output | 8.9572 ms | 6.3790 ms | 28.8% less time |
| Same pixelation, scale 4 | 4.3449 ms | 3.7202 ms | 14.4% less time |

## Verification

- Locked, offline, all-target headless suite: **246 passed, zero failed**; one manual benchmark ignored in the normal suite and separately passed in release mode.
- Exact old/new text, numbered-step and pixelation pixel comparisons passed. Pin close-before-create and close-with-file-open tests passed.
- Independent patch review found no actionable regression. Scoped formatting and whitespace checks passed. Clippy completed with existing style warnings; the two new needless-borrow warnings in pin cleanup were corrected.
- The first full-suite attempt failed renaming an image in the system temporary directory with access denied. The isolated rerun redirected `TEMP`, `TMP` and `SNIPPING_TOOL_UL_CONFIG_DIR` into this pass's workspace audit directory and passed. No save behavior was changed to conceal that environment restriction.
- Whole-repository formatting also reports unrelated concurrent changes in `export.rs` and `bug_pass_0314_tests.rs`; this pass formats only its owned edits.

Commands: `cargo test --offline --locked --all-targets -- --test-threads=1`, `cargo run --offline --locked --release --example review_speed_bench`, and `cargo test --offline --locked --release --lib preview_render::speed_tests::benchmark_oversized_pixelation_fallback -- --exact --ignored --nocapture`.

Baseline source, hashes, the scoped patch and logs are in the parent workspace's `audit/speed-review-20261012/`. Changes remain in source; this pass did not install or publish a binary. Native capture/paste, global input, mixed monitors, actual appearance, whole-app latency and process memory still need isolated Windows acceptance.

## Remaining findings for coordination

These were checked in current source and left as handoffs to avoid overlapping the other active bug pass:

1. **OCR lifetime (P2):** `ocr.rs` still awaits WinRT tasks indefinitely and invokes PowerShell without a total deadline. A hung request retains its worker and temporary PNG. Add a bounded process deadline with kill/reap and synthetic lifecycle tests; the caller should also bound concurrent OCR requests.
2. **History deduplication (P2):** `clear_history()` does not reset `LAST_SAVED_KEY`. Save A, clear, save A can skip the write and leave the gallery empty. The fingerprint check/save/update also spans separate critical sections, permitting concurrent duplicates. Preserve one lock order and serialize this decision with history publication; test against an isolated directory.
3. **Picker stale selection (P2):** shrinking a drag back to zero width/height returns from `update_selection` without clearing the old hole/border. Finishing cancels, but the displayed selection remains stale. Restore the full veil and hide the border on an empty region. Native visual acceptance remains required.

The existing clipboard/startup fixes and other agents' editor/presenter changes are outside this pass's ownership and are not attributed to it.
