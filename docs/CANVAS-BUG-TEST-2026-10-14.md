# Canvas bug test, 2026-10-14

Four corrections completed locally. Two pixel defects were reproduced before changing implementation; two additional integration defects were found by inspecting the export callers.

| Defect | Evidence | Correction |
| --- | --- | --- |
| Fractional pixelation crossing the right/bottom image edge changed its block pixels after canvas growth | Region `(95.9, 73.9, 20.5, 20.5)` on a generated 100 x 80 image failed exact source-pixel comparison at row 73 | Compute the integer source region before moving it into the expanded canvas |
| Fractional spotlight crossing the top/left edge changed restored source pixels after canvas growth | Region `(-2.4, -3.2, 8.7, 9.9)` failed source-pixel comparison at row 0 | Use the same source-region rounding before offset; an entirely outside spotlight still dims the canvas |
| Expanded pins were positioned at the original selection origin, moving original image content by the added margin | Pin caller still used `sel.x/sel.y` despite expanded output | Derive the export origin including the margin; saturating monitor-offset addition; regression checks negative monitor coordinates and cropped selections |
| Oversized canvas preparation could silently abort Copy/Save or clear Upload feedback | Preparation returns `None`; callers lacked a visible failure branch | Copy/Save status and toast, failed Upload feedback and actionable Pin toast; preserve image and annotations |

## Verification

- 348 locked/offline all-target tests passed, zero failures, three existing manual tests ignored. Twenty canvas tests include eight added during this bug test.
- Seventy-two source-effect combinations compare original pixels before/after expansion: six fractional/outside regions, four alpha levels and pixelation/spotlight/both.
- One hundred eighty growth/shrink/offscreen/pan/zoom transitions compare cached rendering with a fresh renderer, then repeat the cached frame. Retained bytes remain within the viewport budget.
- Empty-source preview/export tests passed before fixes; these hypotheses were rejected rather than treated as bugs.
- Same-pattern search covered source clipping, preview, pixelation rasterization, spotlight rasterization, export origins and all four export failure branches. Baseline export/tool rasterizers already use the intended integer rounding; translation was the changed path.
- Formatting, whitespace and personal-identity checks passed. Twenty headless UI renders and the full export were regenerated; expanded dark and resized light/150% proofs inspected.
- Remapped release build completed without compiler warnings. CLI exited 0; binary identity scan found zero personal-name matches.

Candidate: `target/canvas-bugfix-candidate-20261014/snipping-tool-ul.exe`. SHA-256: `8068AAAF703ECE6528A1862A9DFD2D4ACBC8E5F3E49F69CEBCD172338A0C6FA3`. The executable retains the local manifest's 0.3.21 number; it differs from the published and installed runtime. The earlier candidate is preserved separately.

## Correctness Verification

### Requirements: PASS

Growth, shrink and undo/redo derive canvas bounds in `canvas.rs` and `state.rs`. Preview retains a stable image transform; export includes full annotation ink. Pin origin now follows export extents. Existing growth/resize/delete regressions pass.

### Concurrency: PASS

No new shared mutable state or threads. Export preparation owns its pixels/annotations; Save passes the complete snapshot to its existing worker. UI revision/stale-result checks are unchanged.

### Errors: PASS for source integration

Preparation rejection reaches visible failure paths instead of silently continuing. Allocation failure propagates through preparation; image state stays intact. Native presentation of messages remains part of acceptance below.

### Resources: PASS for tested components

Buffers use Rust ownership. Preview retention is bounded by the viewport; full-resolution allocations are checked. Repeated cached/fresh comparisons passed. This is not a native full-process memory measurement.

### Boundaries: PASS

Fractional and negative edges, alpha, outside masks, empty source, huge extents, NaN effect coordinates, cropped selection and negative monitor coordinates covered. Every tool's ink-bounds reference test remains passing.

### Security: PASS for changed scope

Source-effect coordinates must be finite. Export dimensions/byte arithmetic are checked before expansion. No new external actions, permissions or credential paths; build paths are remapped and the candidate identity scan passed.

**Verdict: DONE for local source/headless bug testing.** Native mouse/clipboard/pin-window acceptance still needs isolation. This pass did not install, launch native UI, capture the desktop, upload images, change settings/processes or publish source.
