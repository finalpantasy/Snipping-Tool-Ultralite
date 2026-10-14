# Snipping Tool UL performance

Component measurements from October 12-13, 2026, and a startup and memory comparison with Microsoft's Snipping Tool from October 14, 2026.

[Download and overview](../README.md) | [Release verification](VERIFICATION-0.3.22.md)

## Results at a glance

| Component | Measured result | Comparison |
| --- | --- | --- |
| Integrated local OCR | Small text 17.03 ms; dense 1080p 48.07 ms; dense 4K 211.57 ms | Absolute release component timings |
| Direct OCR invocation | About 3.6-33x faster; 61-92% lower additional peak commit | Earlier paired prototype versus UL's PowerShell path |
| History thumbnail preparation | 98.9% lower peak Rust heap at 4K; 99.7% lower at 8K | Earlier full decode versus streaming thumbnail decode |
| Original-resolution History copy | Approximately half the image heap peak | Earlier cloned RGBA buffer versus consumed buffer |
| Annotated panning | 21.9% less measured processing time | Renderer before and after optimization |
| Pixelation-heavy zoom | 28.2% less measured processing time | Renderer before and after optimization |
| Repeated dense 4K OCR | Two 1,000-call traces succeeded; live Rust allocations and handles stayed flat after warm-up | Component stability, not a full-app leak certification |

The verified 0.3.21 preview includes these implementation changes and passed 328 automated tests, with three manual diagnostics ignored in the ordinary suite. Test counts measure covered behavior, not performance against another application. Later local canvas work is outside this published-runtime summary.

## OCR: release timings and earlier paired comparison

The integrated owned-input OCR path was measured in three fresh release diagnostic processes per successful case. Timings surround the production OCR call, including native setup and conversion; authored image generation is outside the timer. All 15 text cases found their expected markers; blank and malformed inputs returned the expected errors.

| Input | Median OCR call | Additional peak commit | Adjusted retained commit |
| --- | ---: | ---: | ---: |
| Small text, 800 x 300 | 17.03 ms | 6.57 MiB | 1.47 MiB |
| Sparse text, 1920 x 1080 | 26.22 ms | 23.19 MiB | 1.11 MiB |
| Sparse text, 3840 x 2160 | 54.08 ms | 66.19 MiB | 0.91 MiB |
| Dense text, 1920 x 1080 | 48.07 ms | 23.22 MiB | 1.32 MiB |
| Dense text, 3840 x 2160 | 211.57 ms | 66.18 MiB | 2.70 MiB |

An earlier paired prototype used the same generated inputs, three fresh workers per case and warm OS caches. It called the same Windows OCR engine directly, avoiding PowerShell, temporary PNG files and PNG encode/decode. That comparison produced the following values:

| Input | Previous PowerShell call | Direct native prototype | Additional peak commit, previous / native |
| --- | ---: | ---: | ---: |
| Small text | 451.9 ms | 13.8 ms | 80.0 / 6.4 MiB |
| Sparse 1080p | 502.2 ms | 21.9 ms | 103.3 / 23.1 MiB |
| Sparse 4K | 450.9 ms | 50.8 ms | 170.5 / 66.0 MiB |
| Dense 1080p | 422.8 ms | 45.8 ms | 103.4 / 23.0 MiB |
| Dense 4K | 702.3 ms | 195.5 ms | 169.9 / 66.0 MiB |

The 3.6-33x and 61-92% ranges come from this paired prototype table. The later integrated-release table is a separate measurement with production lifecycle handling. Neither baseline is Microsoft's Snipping Tool. OCR accuracy evidence covers the authored cases, not every language or document type. OCR starts only after an explicit action.

Commit means allocated Windows virtual memory, not physical RAM. The peak counter covers the owned diagnostic job, including its hidden console host. Retained commit is an adjusted estimate after the consumed input buffer is freed, not a whole-app footprint.

See [integrated OCR methods, cancellation and retained-memory limits](OCR-RELEASE-VALIDATION-2026-10-13.md). The earlier paired prototype measurements are retained in the local development audit; this table preserves their values and scope.

## History: smaller allocations with original pixels preserved

Fresh release processes measured generated RGBA PNGs, three samples per size and implementation. Fixture creation is outside the measured interval.

| Operation | Input | Previous peak Rust heap | Updated peak Rust heap | Previous / updated median |
| --- | --- | ---: | ---: | ---: |
| Thumbnail preparation | 4K | 31.92 MiB | 0.34 MiB | 43.065 / 38.910 ms |
| Thumbnail preparation | 8K | 126.84 MiB | 0.37 MiB | 203.272 / 158.705 ms |
| Original-resolution copy decode | 4K | 63.28 MiB | 31.92 MiB | 47.185 / 36.301 ms |
| Original-resolution copy decode | 8K | 253.13 MiB | 126.84 MiB | 160.155 / 146.523 ms |

Thumbnail preparation streams ordinary non-interlaced 8-bit PNGs instead of retaining a full decoded image. Other formats retain bounded fallback decoding. History copy consumes its RGBA allocation, preserving original resolution without a second full-image clone. Rapid requests keep one active preparation and only the newest pending request.

These counters measure requested Rust heap bytes. They exclude allocator overhead, native/OS memory and clipboard publication. They are not total process RAM or complete copy-to-paste times. See [thumbnail methods and fallback limits](HISTORY-MEMORY-2026-10-13.md) and [copy measurements and leak investigation](LEAK-INVESTIGATION-2026-10-13.md).

## Rendering: faster measured work

The paired renderer fixture uses generated pixels in one release-optimized process, nine measured samples after two warmups per implementation. Each sample creates its renderer/frame buffer and renders 21 frames into a 1920 x 1080 frame.

| Scenario | Previous median | Updated median | Reduction |
| --- | ---: | ---: | ---: |
| Whole-pixel pan, 12 arrows, 21 frames | 12.736 ms | 9.952 ms | 21.9% |
| Zoom, 12 pixelation regions, 21 frames | 378.663 ms | 271.978 ms | 28.2% |

The implementations performed the same sampling/composition/grid work. These totals are for the fixture's 21-frame sequence, not individual UI frame times. See [paired renderer methodology and pixel-parity checks](OPTIMIZATION-PASS-2026-10-13.md).

Separate generated-image GDI tests measured frozen-image drawing at 0.148 ms for 1080p, 0.985 ms for 4K and 3.775 ms for two adjacent 4K sources. These memory-DIB measurements exclude desktop acquisition, window presentation, compositor synchronization and clipboard work. See [frozen-capture component timings](FROZEN-PERFORMANCE.md).

## Stability: component evidence and remaining limits

Two 1,000-call dense 4K OCR traces completed successfully. After warm-up, requested live Rust storage stayed at 33,191,186 bytes in 10 blocks, including the constant 31.64 MiB authored source. Handles stayed at 131; threads varied between two and five without accumulating. In the final trace, private process memory still ranged from 37,621,760 to 47,456,256 bytes.

Earlier conservative memory-variation guards failed and remain recorded. Longer traces distinguished stable live allocations from fluctuating committed/cache pages; they did not certify a leak-free full GUI application. Diagnostic heap-cache trimming was not added to production. Cancellation, timeout-stop and shutdown fixtures released their owned jobs and temporary files. See the [full investigation](LEAK-INVESTIGATION-2026-10-13.md).

## Microsoft stock Snipping Tool comparison

### Startup and memory: measured

UL 0.3.21 and Microsoft Snipping Tool 11.2608.41.0, same PC and Windows 11 session, October 14, 2026. Each trial started a fresh process with no arguments on a private, never-displayed Win32 desktop, so the launches could not take focus or be disturbed by input. Five warm-up rounds, then 30 measured rounds; the two apps alternated which went first each round. OS file caches were warm, so these are not cold starts.

| Median (p95) | UL 0.3.21 | Microsoft 11.2608.41.0 |
| --- | ---: | ---: |
| Process start to first visible titled window | 47 ms (80) | 235 ms (284) |
| Process start to CPU settled (last 50 ms tick using over 1 ms CPU, then 500 ms quiet) | 125 ms (471) | 378 ms (886) |
| CPU time used by then | 94 ms (141) | 469 ms (594) |
| Private bytes, 1 s after settling | 10.2 MB | 101.8 MB |
| Working set, 1 s after settling | 37.5 MB | 130.0 MB |
| Handles / threads | 450 / 19 | 1,297 / 97 |
| CPU over 10 s idle with the window open | below counter resolution | below counter resolution |
| Package size on disk | 10.0 MB executable | about 420 MB (586 MB logical, hard-linked duplicates), plus Windows App Runtime and WinUI dependencies |

Three of the 60 measured launches (two UL, one Microsoft) showed no window within 20 s and are excluded. They coincided with a second benchmark running on the same machine, which also stopped processes by name. Part of the run overlapped that load, so absolute times may be slightly high; the ratio is consistent with the second harness, which launched on the visible desktop and measured 62.6 ms (p95 88.6) for UL and 256.7 ms (p95 392.1) for Microsoft.

A visible window is not the same as a fully drawn, input-ready window; the CPU-settled time is the closer proxy. UL also keeps a tray process when its window is closed (about 2 MB private, 12 MB working set, effectively no CPU); Microsoft's app keeps no resident process.

### Still to measure

Capture-to-editor, capture-to-clipboard, OCR and sustained use have not been compared. A fair comparison requires both applications in the same Windows environment:

| Measurement | Matched procedure |
| --- | --- |
| Startup | Record exact app/Windows versions; measure new-process startup and warm reopen separately, with the same visible-and-ready criterion |
| Idle resources | Measure private commit, working set, CPU-time delta, handles and threads over equal idle intervals; separate open editor and background/tray states where supported |
| Region capture | Use the same authored 1080p/4K desktop and fixed region; time selection activation and completed selection-to-editor separately |
| Clipboard readiness | Measure completed selection-to-successful bitmap retrieval, including encoding/publication; verify dimensions and image content |
| OCR | Use identical authored images and check output markers; include the complete user-action-to-result time separately from any exposed engine timings |
| Sustained use | Repeat the same capture/edit/copy/close sequence; compare warm baselines and post-cleanup resource trends |

Keep display scale, monitor layout, capture mode and equivalent features consistent. Record defaults first; report any altered settings, such as history, auto-save or automatic OCR. Alternate application order, retain per-trial results, and report sample counts, medians and p95 alongside failures. Use at least 30 measured trials after five warmups for repeated latency comparisons. Record the cold-start reset method separately; a fresh process alone does not establish cold OS caches.

Results from synthetic UL components cannot substitute for that comparison. Native mixed-monitor behavior, full interactive memory and end-to-end latency remain separate acceptance work.
