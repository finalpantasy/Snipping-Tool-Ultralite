# Snipping Tool UL 0.2.0 verification

2026-10-12. Native Rust graphite/frosted redesign, automatic copy then preview, and setup-free sharing. The 0.1.0 package and its verification remain preserved as historical evidence.

## Verified

- 162 headless tests passed, zero failures; cargo fmt check passed and remapped portable release built.
- Actual home/editor raster output inspected at 100% and 150% DPI, including tool/options/color menus, connection and zoom. These are synthetic application client renders, not desktop screenshots or DWM composition captures.
- All original annotation/export actions remain reachable, including custom color, OCR and pin. Pending text commits before copying/export. Menu input cannot draw through to the image. Pointer zoom geometry checked at 100/125/150/200% scales.
- Anonymous multipart upload payload and returned-link validation tested without sending images. Redirects are disabled and response text is bounded. Existing clipboard supersession remains in the upload path. Default Catbox upload requires no client ID; an explicitly configured Imgur ID selects Imgur without silent provider fallback.
- RES source reviewed: its default image handler recognizes direct paths ending in PNG. This supports the chosen link format; it is not a live Reddit/RES interaction result.
- Impeccable detector returned no findings on changed native rendering files. Its regex-based checks do not prove native accessibility. Final source and interaction review was parent-owned; delegation routing did not authorize an independent reviewer.

## CPU rendering measurements

Release helper, synthetic image, 1410 by 918 physical editor viewport at 150% DPI. Median of 21 CPU frames; excludes native presentation, capture, PNG encoding, clipboard, annotation-heavy workloads and uploads.

| Work | CPU time | Retained image/render/viewport pixel bytes |
|---|---:|---:|
| Home warm median of 30 | 0.103 ms | 982,080 |
| 960 by 540 editor | 3.973 ms | 9,324,720 |
| 1920 by 1080 editor | 4.156 ms | 21,766,320 |
| 3840 by 2160 editor | 5.037 ms | 71,532,720 |

Home first CPU render was 23.652 ms. Pixel-byte counts exclude native surfaces, fonts, caches, allocator overhead and the rest of the process; they are not total RAM. These results are single-machine observations, not performance guarantees.

## Private native presentation probe

The executable refuses any desktop whose name does not start with the private test prefix, before creating its event loop. The harness creates a private Win32 desktop, assigns only the synthetic probe to it and never switches the input desktop. Default-desktop refusal returned exit 2. Ten open/present/close cycles succeeded on the private desktop; input desktop stayed unchanged. No desktop capture, clipboard writes, hotkeys, tray, notifications or uploads ran.

The initial hardware presenter reached 86.93 MiB on its first open and 93.88 MiB by its tenth; first presentation was 414.458 ms. The selected software Direct2D presenter measured 58.92 MiB first and 59.98 MiB tenth, with 219.344 ms first presentation and 17.457 ms median subsequent presentation. This was an optimized development helper, not the full app. Font and native library caches remain resident after closing a surface. Ten cycles show no comparable continuing driver-cache growth, but do not establish long-term leak freedom.

Native backdrop requests and presentation succeeded. An inactive private desktop cannot establish how acrylic looks on an active composited desktop. Real capture-to-clipboard latency and total product memory remain unmeasured.

## Remaining acceptance

Actual region/window capture, real image paste into receivers, timers/hotkeys, mixed-monitor GUI, live Catbox/Imgur success and retry, Reddit/RES expansion, active native glass appearance and Start search require an isolated Windows session or user-directed use. No test images were publicly uploaded. The Start shortcut remains default-dry-run; no install, startup registration, commit, remote, push or publication occurred. The UI's custom raster controls have not been audited for screen-reader semantics.

Package integrity and identity checks are recorded in the release manifest and local audit report. Use scripts/Build-Portable.ps1 for distributable builds so machine-specific compiler paths are remapped.
