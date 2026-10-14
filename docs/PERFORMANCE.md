# Snipping Tool UL performance

Component measurements from October 12-13, 2026, and capture, startup and memory comparisons with Microsoft's Snipping Tool, ShareX, Greenshot and Flameshot from October 14, 2026.

[Download and overview](../README.md) | [Release verification](VERIFICATION-0.3.25.md)

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

## Comparison with ShareX, Greenshot and Flameshot

Measured October 14, 2026 on the same PC and harness as the Microsoft comparison below: UL 0.3.25, Microsoft Snipping Tool 11.2608.41.0, ShareX 21.0.0, Greenshot 1.3.323 and Flameshot 14.0.0, all official portable builds with verified download digests. Background load from other applications was present (CPU 44 to 74% before the run) and affected all apps equally; absolute times are a little higher than in the quiet October 14 Microsoft-only run.

**Settings.** Every app copied each snip to the clipboard and saved it to disk: Microsoft's default auto-save, UL with History on, ShareX's defaults (copy and save; no upload), Greenshot with its destination picker replaced by clipboard plus file. Hotkeys: Win+Shift+S (Microsoft), Ctrl+Shift+S (UL), Ctrl+Shift+F10 (ShareX; its default Ctrl+Print Screen was taken on this PC), Ctrl+Shift+F9 (Greenshot; its default Print Screen is Windows' own key).

**Method.** Five warm-up rounds, then 30 measured rounds. The app order rotated each round so no app ran twice in a row. Each trial waited 800 ms after the selection screen appeared before dragging, so every app was ready for input; the timings below do not include that wait.

### Region capture

| Median (p95) | UL | Microsoft | ShareX | Greenshot |
| --- | ---: | ---: | ---: | ---: |
| Hotkey to selection window visible | 170 ms (174) | 363 ms (434) | 249 ms (284) | 265 ms (283) |
| Release to bitmap readable from the clipboard | 50 ms (74) | 91 ms (616) | 144 ms (161) | 63 ms (65) |
| Release to editor window visible | 94 ms (111) | none | none | none |
| Private bytes of the app when the bitmap was readable | 14 MB (29) | 474 MB (475) | 922 MB (1,676) | 133 MB (137) |
| Peak working set of the app | 288 MB | 498 MB | 2,522 MB | 506 MB |
| Correct size and pixels | 30 of 30 | 30 of 30 | 29 of 30 | 28 of 30 |

ShareX's private memory rose by about 105 MB with every snip and only fell back when .NET garbage collection ran, after reaching roughly 1.1 to 1.9 GB, so its value depends on how many snips came before. The three wrong snips (one ShareX, two Greenshot) all came out 1175 x 661, consistent with the selection starting at the first mouse-move point instead of the press point. In an earlier attempt that let the same app snip twice within about 3 seconds, Greenshot missed the press in every second snip; rotating the order removed that pattern.

**Flameshot** was not measured for capture. On a PC with several monitors it opens a monitor picker before every snip, even with its `captureActiveMonitor` setting on, and its only Windows hotkey is Print Screen, which Windows keeps for its own tool by default.

### Background cost while waiting in the tray

Each app started fresh on a private, never-displayed desktop; 10 measured launches after 2 warm-ups, plus 3 ten-second idle samples.

| Median | UL | ShareX | Greenshot | Flameshot |
| --- | ---: | ---: | ---: | ---: |
| Start to CPU quiet (1 s with no work) | 78 ms | 1,328 ms | 327 ms | 202 ms |
| CPU time used starting | 16 ms | 922 ms | 313 ms | 94 ms |
| Private bytes, idle in the tray | 2.0 MB | 107 MB | 45 MB | 8.3 MB |
| CPU over 10 s idle | none measurable | 0 to 47 ms | none measurable | none measurable |

Microsoft's Snipping Tool keeps no process running between snips, so its background cost is zero.

### Size

| | UL | Microsoft | ShareX | Greenshot | Flameshot |
| --- | ---: | ---: | ---: | ---: | ---: |
| Download | 4.9 MB | Store package | 199 MB | 3.5 MB | 53 MB |
| On disk | 9.6 MB | about 420 MB | 517 MB | 9.0 MB | 95 MB |

Greenshot is the smallest on disk because it relies on the .NET Framework that ships with Windows. ShareX's folder includes ffmpeg (193 MB) for screen recording.

## Microsoft stock Snipping Tool comparison

UL 0.3.22 and Microsoft Snipping Tool 11.2608.41.0, measured October 14, 2026 on one Windows 11 PC (16-core desktop CPU, 64 GB RAM) with three 3840 x 2160 monitors at 150% scaling, one at a negative origin. Both apps used their own capture hotkeys and their normal settings: Microsoft saved each snip to Pictures\Screenshots (its default) and UL kept History on, so both wrote every snip to disk. Five warm-up rounds, then 30 measured rounds; the apps alternated which went first each round. File caches were warm, so none of these are cold starts.

### Region capture

A topmost window filled the primary monitor with a known tile pattern. Each trial cleared the clipboard, sent the app's hotkey (Win+Shift+S for Microsoft, Ctrl+Shift+S for UL), waited for a new window covering the primary monitor, waited a fixed 300 ms, then dragged from (600, 400) to (1880, 1120) in physical pixels and released. The clipboard bitmap was read back and five interior pixels checked against the pattern.

| Median (p95) | UL 0.3.22 | Microsoft 11.2608.41.0 |
| --- | ---: | ---: |
| Hotkey to selection window visible | 155 ms (160) | 355 ms (382) |
| Release to bitmap readable from the clipboard | 50 ms (53) | 76 ms (601) |
| Release to editor window visible | 78 ms (80) | none; Microsoft shows a notification |
| Private bytes when the bitmap was readable | 29 MB (39) | 484 MB (502) |
| Peak working set of the capturing process | 288 MB | 514 MB |
| Processes left after the snip | 1 (tray, about 43 MB private) | 0 |
| Correct size and pixels | 29 of 30 | 30 of 30 |

Image sizes were 1280 x 720 for UL and 1279 x 720 for Microsoft; the apps treat the end pixel differently. Microsoft's p95 clipboard time reflects occasional 600 ms waits. UL's peak working set includes its frozen copy of all three 4K monitors.

**Bug found and fixed by this test.** When the pointer kept moving immediately after release, UL committed the later pointer position: a release followed 1 ms later by a 300 px move captured 1580 x 1020 in 9 of 10 trials, while Microsoft was correct in 10 of 10. The picker now uses each mouse message's own coordinates; the same test then gave 15 of 15 exact results with unchanged timings. In the main run, one measured UL trial (iteration 20) came out 1570 x 887: the start corner was exact, but the end corner was a point the harness never sent. Its cause is not confirmed.

### Startup and memory

Each trial started a fresh process with no arguments on a private, never-displayed Win32 desktop, so launches could not take focus or be disturbed by input. All 60 measured launches succeeded.

| Median (p95) | UL 0.3.22 | Microsoft 11.2608.41.0 |
| --- | ---: | ---: |
| Process start to first visible titled window | 47 ms (78) | 235 ms (265) |
| Process start to CPU settled (last 50 ms tick using over 1 ms CPU, then 500 ms quiet) | 125 ms (426) | 374 ms (720) |
| CPU time used by then | 94 ms (156) | 469 ms (609) |
| Private bytes, 1 s after settling | 10.3 MB | 102.0 MB |
| Working set, 1 s after settling | 37.5 MB | 130.5 MB |
| Handles / threads | 450 / 19 | 1,323 / 99 |
| CPU over 10 s idle with the window open | below counter resolution | below counter resolution |
| Package size on disk | 10.0 MB executable | about 420 MB (586 MB logical, hard-linked duplicates), plus Windows App Runtime and WinUI dependencies |

An earlier run on UL 0.3.21 gave the same medians within 1 ms and 0.2 MB. A second harness, launching on the visible desktop, measured 62.6 ms for UL and 256.7 ms for Microsoft. A visible window is not the same as a fully drawn, input-ready window; the CPU-settled time is the closer proxy.

### Still to measure

OCR, sustained use over long sessions, cold starts after a reboot, and single-monitor or 100% scaling setups have not been compared.
