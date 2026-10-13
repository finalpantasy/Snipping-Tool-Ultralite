# Verification evidence

Historical 0.1.0 evidence. Current 0.2.0 results are in VERIFICATION-0.2.0.md; the current headless benchmark helper now renders the redesigned UI.

2026-10-12, Windows x64. Release Rust build, locked dependencies. All activity in this phase was headless. Synthetic image data only; no working-desktop captures, GUI fixtures, system clipboard writes, public uploads or installation changes.

`cargo test --locked`: 153 tests passed, zero failed. Coverage includes annotation/export behavior, negative desktop origins, geometry clipping, launcher actions, workflow defaults, timer choices, home controls rasterized at physical display scale, pointer-centered zoom at 100/125/150/200 percent DPI, footer placement on large and tiny captures, original pixel preservation, image validation and stale clipboard write rejection. `cargo fmt --check` and the release build passed.

The headless renderer's home, upload connection panel and synthetic editor previews were visually inspected. These are the application's rendering code, but do not prove native HWND input, composition, focus, window styling or real monitor behavior.

## Synthetic CPU benchmark

Release `examples/headless_bench.exe`, first home frame once; subsequent home frames median of 30. Editor frames median of 21, source image plus render target plus 1200x800 presentation buffer, at a simulated 150 percent display scale and zoomed view.

| Work | CPU time | Retained pixel buffers |
| --- | ---: | ---: |
| Home, first frame | 5.031 ms | 1,440,000 bytes |
| Home, warm frame | 0.355 ms | 1,440,000 bytes |
| Editor, 960x540 image | 3.431 ms | 8,290,944 bytes |
| Editor, 1920x1080 image | 4.918 ms | 20,920,320 bytes |
| Editor, 3840x2160 image | 9.812 ms | 71,178,240 bytes |

Times cover CPU rendering and presentation-buffer conversion only. They exclude process startup, Windows capture, actual clipboard encoding/transfer, native surface presentation and network upload. Buffer totals exclude executable pages, allocator overhead, fonts, native surfaces and other process memory. They are not total RAM measurements. Source pixels and rendering buffers grow with the selected image, while zoom never creates an enlarged image buffer. Idle uses an event wait with callback wakeups rather than a permanent frame loop.

Start shortcut script: PowerShell syntax passed; its default dry run was executed and verified not to create a shortcut. Search result ranking and actual registration have not been tested.

## Remaining native acceptance checks

Use an isolated Windows VM/session. Confirm home reopening from Start and tray, hotkey conflicts, timer cancellation, region/window/full-desktop capture, mixed-DPI monitors and negative origins, pointer alignment and export resolution after zoom, real paste into target chats, busy clipboard retries, save dialogs, optional upload with failed-network retry, and memory across repeated capture/close cycles. Keep the working desktop untouched. Actual total working set and capture latency for this new product build remain unmeasured.

The earlier modified HydroShot development baseline measured a 34.833 ms median picker appearance and an observed 55.25 MiB peak for a 1918x401 capture/edit workload. These are historical baseline measurements, not measurements of this packaged build; the original fixture files and evidence remain outside this independent repository.
