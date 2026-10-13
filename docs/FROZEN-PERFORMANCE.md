# Frozen capture performance

Measured on 2026-10-13 using generated pixels and memory-only Windows GDI targets. No screen was captured and no windows, clipboard operations or uploads were used. This source update is not installed.

## Result

The initial painter let GDI convert arbitrary RGBA bitfields on each redraw. It was correct, but slow in this fixture. The replacement converts the existing buffer in place at entry, paints native BGRA, and restores RGBA on exit. It adds no full-screen CPU buffer or dependency. An exclusive borrow and cleanup guard keep the conversion private to selection and restore the pixels on errors or unwinding.

| Source size | Initial draw median | Native draw median | Native draw p95 | Source pixel buffer |
| --- | ---: | ---: | ---: | ---: |
| 1920 x 1080 | 23.051 ms | 0.148 ms | 0.371 ms | 7.91 MiB |
| 3840 x 2160 | 94.969 ms | 0.985 ms | 1.302 ms | 31.64 MiB |
| 7680 x 2160, two adjacent 4K monitors | 200.048 ms | 3.775 ms | 4.867 ms | 63.28 MiB |

| Source size | Entry plus exit conversion median | Conversion p95 | 800 x 600 snip median | Snip p95 |
| --- | ---: | ---: | ---: | ---: |
| 1920 x 1080 | 0.852 ms | 1.274 ms | 0.370 ms | 0.481 ms |
| 3840 x 2160 | 3.355 ms | 4.789 ms | 1.028 ms | 1.366 ms |
| 7680 x 2160 | 7.118 ms | 8.906 ms | 1.801 ms | 2.212 ms |

Entry/exit conversion is paid once each per selection, rather than each redraw. The snip timings include release of the consumed desktop buffer. Full-screen selection reuses its buffer and took less than 0.001 ms in the metadata-only crop call; this excludes acquisition and display.

## Method and limits

- Release-optimized production painter and crop methods, same host and source sizes before and after, three warmups and 31 measured samples per component. Reported p95 uses the nearest-rank sample. No claim of statistical confidence or universal timings is made.
- Drawing includes `StretchDIBits` and `GdiFlush` into an existing memory DIB. DIB allocation is excluded. This validates component cost, not actual on-screen window presentation or driver/compositor behavior.
- Authored image construction and cloning are outside the timed crop/conversion calls. Output disposal is outside the crop timer. Input memory is recently touched, so the fixture does not model cold caches or every competing workload.
- Desktop acquisition, DWM synchronization, window creation, clipboard encoding and publication, preview startup, and hotkey-to-selection latency are not measured here. Sum of component medians is not an end-to-end measurement.
- Pixel-buffer sizes are exact capacities in this fixture, not total application RAM or committed-memory peaks. Capture and display also use Windows/GDI/compositor resources. The source buffer is temporary and is released after region selection or cancellation; only the selected image remains.

Reproduce explicitly with `cargo test --release --locked --offline --lib benchmark_frozen_capture_costs -- --ignored --nocapture`. The benchmark remains ignored during ordinary test runs and creates no HWND or screen DC.

Raw before/after logs are retained locally in `audit/frozen-capture-20261013/performance-before.log` and `performance.log`. Actual frozen selection behavior and latency remain part of the deferred isolated Windows acceptance gate.

Following the optimization, the full locked/offline all-target suite passed **301 tests, zero failed, two ignored manual tests**, without compiler warnings. This includes the production GDI color test and buffer restoration after errors and unwinding. Formatting, diff and scoped public-identity checks passed. Nothing was installed or published.
