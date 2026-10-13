# Memory investigation, 2026-10-13

The generated-input OCR trace separates live allocations from Windows heap caching. It also exposed two reproducible application problems and one fault-injected native ownership gap. All validation described here ran headlessly. The installed 0.3.14 application was not replaced.

## Findings and fixes

- Rapid History copy requests previously created a worker and decoded a full image for every click, even when a newer request superseded them. A channel-gated reproduction started 20 concurrent payloads. Preparation now keeps one active request and only the latest pending request, checks freshness before and after decoding, and exits when idle. Its worker is included in shutdown tracking.
- History copying previously retained the decoded image while cloning its entire RGBA buffer. The new original-resolution decoder consumes an existing RGBA allocation. Encoded-file and decoded-image limits reject oversized inputs. The 128 MiB limit applies to the decoded source and RGBA output individually; it is not a total process-memory ceiling. Other colour conversions, codec workspace and clipboard publication can require additional memory.
- OCR cancellation could wait indefinitely behind another clipboard writer. A mocked held-writer reproduction failed before the fix. Clipboard admission now observes cancellation and supersession while waiting. Quit uses one absolute 15-second deadline for the OCR caller and other accepted background work.
- A failing native OCR status query could release the single-flight lease without evidence that Windows had finished using the image. A fault-injected reproduction failed before the fix. The caller now receives the error promptly, while the worker retains the image and lease until a terminal status is observed. Repeated errors send cancellation once and cannot admit a second worker or produce late success. An actual Windows status-query failure was not observed. If Windows never reports a terminal state, at most one worker remains reserved until process exit.

## Measured History copy improvement

Authored RGBA PNGs, original resolution preserved, three fresh process runs per mode. These counters measure requested Rust heap bytes for decoding and retained image buffers, excluding the fixture-generation phase and OS/clipboard allocations.

| Image | Previous heap peak | New heap peak | Previous median | New median |
| --- | ---: | ---: | ---: | ---: |
| 3840 × 2160 | 63.28 MiB | 31.92 MiB | 47.185 ms | 36.301 ms |
| 7680 × 4320 | 253.13 MiB | 126.84 MiB | 160.155 ms | 146.523 ms |

The tracked image allocations returned to zero after decoding in every benchmark run. The result is approximately half the previous image heap peak, with no thumbnail substitution or resolution loss.

## OCR fluctuation attribution

The initial 1,000 dense 4K recognitions all succeeded. After warm-up, requested live Rust storage stayed exactly at 33,191,186 bytes in 10 blocks, including the constant 31.64 MiB authored input. OS handles stayed at 131; threads varied between two and five without accumulation. Native heap live allocations varied by about 55 KiB, while committed heap and private process memory varied by several MiB. This does not establish a leak-free full application or attribute every small native allocation.

A separate diagnostic-only heap-cache experiment reclaimed 10,174,464 private bytes, about 9.70 MiB, in 1.04 ms. Native heap live allocations were unchanged. Instrumentation itself added about 6 KiB of Rust allocations. This provides evidence that free/cache pages explain a material part of the fluctuation. Microsoft documents [HeapOptimizeResources](https://learn.microsoft.com/en-us/windows/win32/api/heapapi/nf-heapapi-heapsetinformation) as decommitting eligible cached heap blocks; [HEAP_SUMMARY](https://learn.microsoft.com/en-us/windows/win32/api/heapapi/ns-heapapi-heap_summary) distinguishes allocated and committed bytes.

No heap-cache trimming or diagnostic allocator was added to the application. Reclaiming the cache on every capture would sacrifice reuse and could hurt latency. The earlier strict memory-variation guard failure is preserved in the audit evidence; stable live allocations do not make every RAM fluctuation disappear.

The final updated-source 1,000-call dense 4K trace also completed successfully. Requested live Rust storage stayed at 33,191,186 bytes in 10 blocks after warm-up. Handles stayed at 131; threads ranged from 2 to 5. Native heap allocated bytes ranged from 33,395,950 to 33,452,170, while private process memory ranged from 37,621,760 to 47,456,256 bytes. All heap-census calls succeeded; the owned job was empty after exit and contained no temporary files. Memory still fluctuated, but live Rust allocations and handles did not accumulate in this component test.

## Verification and boundaries

The locked, offline all-target suite passed 321 tests with zero failures; three manual diagnostics remained ignored. The final run reported an incremental-cache hard-link fallback to copying, rather than a source-code diagnostic. Rust 1.89 binary compatibility and formatting checks passed.

A 1,000-request burst verified one active payload and only the latest pending preparation. A separate 50-batch, 1,000-request test verified release of all generated payloads and no handle growth, with 87 handles before and 86 afterward. Cancellation is signalled only after the mock writer's second admission attempt proves it was blocked. Oversized headers, corrupt files, pixel/alpha parity, RGBA storage reuse and the shared shutdown deadline are covered.

Fresh generated-image native OCR fixtures passed 12 repeated recognitions, 12 coloured recognitions, cancellation, timeout and shutdown. Actual async stop cleanup took approximately 6–9 ms in those fixtures, with empty owned process jobs and no temporary files. The status-query regression tests exercise injected failures, rather than an observed OS failure.

An independent blind read-only review identified the original issues. A separate review of the fixes found no actionable remaining defect; its queue-tracking and test-scheduling suggestions were applied. All existing unrelated work was preserved.

The full GUI event loop, capture, clipboard publication, tray, hotkeys, mixed-monitor DPI and long interactive editing remain unverified in an isolated native session. No test opened the application on the working desktop, captured user pixels, touched the OS clipboard, uploaded an image or changed the installed version. Windows Sandbox enablement remains deferred by the user.

Raw counters, before-fix failures, benchmarks, source hashes and scoped verification are retained outside the repository in the local `audit/leak-investigation-20261013` directory.
