# Integrated OCR release validation

The final 0.3.14 owned-input OCR path was exercised headlessly on Windows on 2026-10-13. Tests used authored text pixels in hidden, owned processes with isolated temporary folders. No working-desktop windows, capture, clipboard, tray, hotkey registration, notifications or network activity were invoked. The installed app was not modified.

## Release measurements

Each successful case uses three fresh release diagnostic processes. Timings surround the production `extract_text_owned` call; authored image generation occurs before timing. The worker runs at below-normal priority, so host load can affect these results. All 15 text cases found the expected markers. Blank and invalid input returned the expected errors, giving 17 completed cases.

| Authored input | Median OCR call | Additional peak commit | Adjusted retained commit |
| --- | ---: | ---: | ---: |
| Small text, 800 x 300 | 17.03 ms | 6.57 MiB | 1.47 MiB |
| Sparse text, 1920 x 1080 | 26.22 ms | 23.19 MiB | 1.11 MiB |
| Sparse text, 3840 x 2160 | 54.08 ms | 66.19 MiB | 0.91 MiB |
| Dense text, 1920 x 1080 | 48.07 ms | 23.22 MiB | 1.32 MiB |
| Dense text, 3840 x 2160 | 211.57 ms | 66.18 MiB | 2.70 MiB |

Commit is allocated Windows virtual memory, not physical RAM. Additional peak is the owned job's lifetime high-water mark minus its ready-state private-commit baseline. The job contains the diagnostic and its hidden console host. A lifetime counter includes fixture preparation; the OCR peaks exceeded that earlier preparation peak in the successful cases. Ten-millisecond private-memory samples are also retained, but can miss brief peaks.

The ready baseline contains the input RGBA buffer. OCR consumes and frees that buffer, so adjusted retained commit adds its known byte count back to the post-call difference after one second. This approximates remaining overhead; it is not a whole-app memory measurement or a precise attribution of every native allocation. The fixture's font setup and observer are outside the timed call. No comparison with Windows Snipping Tool is claimed.

## Cancellation and ownership

The explicit real-Windows unit fixture exercised 12 repeated and 12 colored-text recognitions, cancellation after `RecognizeAsync` started, timeout-stop signaling after recognition started, and public shutdown cancellation. All five fixture processes passed, released their owned jobs and left their temporary folders empty.

- Cancellation: caller stopped and backend ownership released in 7.60 ms.
- Timeout-stop signaling: cleanup completed in 7.76 ms.
- Shutdown cancellation: cleanup completed in 7.17 ms.

The timeout fixture injects the timeout stop reason after recognition starts. The existing independent fake-backend test covers the elapsed deadline, retry exclusion while cleanup is blocked and late-result suppression. These measurements do not guarantee cancellation latency for every Windows backend failure. A backend that does not settle can retain one worker until completion or process exit, while the supervisor keeps later requests from overlapping it.

## Repeated-use investigation

An early 65-call 4K diagnostic exceeded an 8 MiB retained-growth guardrail by reaching an 8.54 MiB increase, with flat handle counts. That producer parsed the font and rasterized a new authored image on every call. A fixture-only control, executing **zero OCR calls**, retained 3.81 MiB after 65 repetitions. Thus the initial measurement included producer/allocator churn as well as OCR.

The diagnostic now keeps a constant authored source during repeated runs and supplies a fresh owned pixel buffer to each OCR request. No production caching or OCR worker behavior was changed. A 65-call small-image run completed with flat handles. The cached 4K run showed fluctuations and still triggered a conservative variation guard, so testing continued to **200 dense 4K recognitions**.

All 200 recognitions succeeded. Sampled job private commit, including the constant 31.64 MiB authored source and hidden console host, ranged from 37.55 to 45.24 MiB after warmup. It returned to 37.55 MiB at call 200, **0.23 MiB below the call-20 baseline**. Handles stayed at 232. The final job had zero active processes and no temporary files.

The raw harness still exits nonzero because its strict last-three-samples variation threshold is 4 MiB; the observed variation was 7.68 MiB. This threshold failure is preserved in the audit rather than reported as a passed soak. The longer run did not reproduce sustained accumulation, and its decline back to baseline argues against treating the earlier endpoint as a confirmed leak. Exact attribution to native caches versus allocator retention remains unproven. A full-app long-session test remains necessary.

## Compatibility and next gate

The production binary passes the declared Rust 1.89 minimum with a locked/offline Windows compile check. Production OCR source and the installed 0.3.14 executable hashes are unchanged. Only a generated-input diagnostic and this report were added; the preceding 312-test suite was not rerun because runtime source did not change.

The offline Sandbox kit was refreshed to the verified 0.3.14 package at `target/sandbox-0314-ready`. Its executable hash matches the installed release. Input is mapped read-only, Results is the only writable mapping, and networking, host clipboard, audio/video input and printers are disabled. Preparation/escaping/overwrite-refusal/host-guard checks pass in PowerShell 7 and Windows PowerShell 5.1. Nothing was enabled or launched.

Native UI capture/paste, mixed-monitor DPI, timer/hotkey interaction, providers, long-session app memory and real working README media still require an isolated Windows session. Local logs, samples, controls, failed guard outputs and final hashes are retained in `audit/release-validation-20261013`.
