# 0.3.9 arrow shape correction

The former renderer stroked the shaft all the way to the tip, then filled the head on top. Its rectangular end remained visible beside the tip, especially at large widths. A headless pixel regression reproduced this before the fix.

Arrows now use a single filled silhouette, with a head six stroke widths across and four widths long. Short arrows cap head length to the actual drag distance and shrink its width proportionally. This removes the protrusion and translucent overlap seams, with one fill and fixed-size outline vertices instead of a separate heap-allocated head. Preview and copy/save exports use the same renderer. Selection and bounds share that geometry so head wings are included.

Five new headless tests cover tapered tips in multiple directions and widths, translucent alpha without double blending, visible pixels contained in selection bounds across short/long arrows, degenerate inputs and selecting head wings. The full locked all-target suite passes 206 tests; formatting and Clippy without blocking errors pass (pre-existing style warnings remain). The production renderer was inspected in before/after synthetic proof images. These are authored pixels, not native screenshot-session media.

A remapped 0.3.9 build and package are verified through the local audit: embedded resources/pixel buffers, source/executable identity scan, safe CLI paths, archive contents/hashes, installed executable metadata/hash and Start shortcut/icon. Existing app processes are preserved and the update is not launched. Private presentation testing from 0.3.8 remains prior evidence; it was not repeated for this raster-only correction.

Claude's local baseline commit 0f83793 contains both agents' work through 0.3.8. The arrow correction is an uncommitted change on top; no commit, remote, push or GitHub publication was performed for this update. Native capture/paste, actual UI appearance/interactions, live hosts and genuine working README media remain separate acceptance gates. See [prior verification](VERIFICATION-0.3.8.md).
