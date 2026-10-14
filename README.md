<p align="center"><img src="assets/ul-icon.png" width="88" alt="Snipping Tool UL teal bird and scissors logo"></p>

<h1 align="center">Snipping Tool UL</h1>
<p align="center"><strong>Ultralight in performance, not features.</strong><br>A lightweight native Rust snipping tool for Windows 11.</p>

<p align="center"><a href="https://github.com/finalpantasy/Snipping-Tool-Ultralite/releases/download/v0.3.22-preview/Snipping-Tool-UL-0.3.22-Windows-x64.zip"><strong>Download Windows preview</strong></a> · <a href="docs/USAGE.md">User guide</a> · <a href="LICENSE">MIT license</a></p>

![Snipping Tool UL light-mode editor](docs/media/editor-light.png)

Take a snip. It is copied immediately, and the preview opens automatically. Paste it into a conversation or keep editing without another button click.

Draw past any image edge and the canvas grows with white space. Move, resize or delete annotations to shrink unused space again. Copy, Save, Upload and Pin include the complete canvas.

**Native Rust. No webview. No permanent render loop.** Teal frosted controls, a themed title bar, and light mode by default, with optional dark mode.

## The essentials, within reach

| Capture | Edit | Keep and share |
| --- | --- | --- |
| Region, window, full screen | Pencil, arrows, shapes, highlights | Immediate image clipboard copy |
| Frozen snapshot during selection | Text, pixelation, numbered steps | Save at original resolution |
| Configurable hotkeys | Live width and size controls | Explicit one-click public image links |
| 3, 5 or 10-second timers | Undo, redo, pointer-centered zoom | Optional history and auto-save |
| Windows Start integration | Middle-mouse pan, local OCR | Pin screenshots on screen |

Catbox, Freeimage.host and temporary Litterbox sharing need no account or personal API key. Uploading is an explicit action, with an optional confirmation and five-second cooldown. An unchanged image reuses its recent successful link. These are public links; [read the hosting details](docs/UPLOAD-HOSTS.md).

<details>
<summary>Home keeps capture controls together</summary>

![Compact light-mode home window](docs/media/home-light.png)

*The launcher keeps capture controls and the screenshot shortcut visible without a large blank canvas.*

</details>

<details>
<summary>Upload shows progress and tells you when the link is ready</summary>

![Light-mode upload feedback](docs/media/upload-light.png)

*The button shows progress, then confirms **Link copied**. If copying fails, **Copy link** retries using the existing upload.*

</details>

## Start using it

1. [Download the Windows x64 ZIP](https://github.com/finalpantasy/Snipping-Tool-Ultralite/releases/download/v0.3.22-preview/Snipping-Tool-UL-0.3.22-Windows-x64.zip) and extract it.
2. Launch `snipping-tool-ul.exe`.
3. Choose **New** or press **Ctrl+Shift+S**.

Portable use needs no installer. **Options > Quit Snipping Tool UL** exits completely. Choose **Keep in tray** or **Quit completely** for the X button in Settings. History and auto-save are off by default.

Optional **Settings > Windows > Use as my Snipping Tool** installs for your account and can replace Microsoft's Snipping Tool entry in Start. The installer explains and confirms the replacement. Windows' reserved Win+Shift+S shortcut is not remapped. [Setup and usage guide](docs/USAGE.md)

## Small by design, measured honestly

The measurements below use generated images on Windows. **Before/after gains compare UL with its own earlier implementations.**

| Measured component | Result |
| --- | --- |
| Integrated local OCR | **17 ms** for small text; **48 ms** for dense 1080p; **212 ms** for dense 4K |
| Earlier paired OCR prototype | **3.6-33x faster** than UL's PowerShell path, with **61-92% less additional peak committed memory** |
| 4K History thumbnail | Peak Rust heap **31.92 to 0.34 MiB**, a **98.9% reduction** |
| 8K History thumbnail | Peak Rust heap **126.84 to 0.37 MiB**, a **99.7% reduction** |
| Original-resolution History copy | Peak image heap **63.28 to 31.92 MiB at 4K** and **253.13 to 126.84 MiB at 8K**, preserving original pixels |
| Annotated panning | **12.736 to 9.952 ms** for a 21-frame fixture, **21.9% less processing time** |
| Pixelation-heavy zoom | **378.663 to 271.978 ms** for a 21-frame fixture, **28.2% less processing time** |
| Frozen-image drawing | **0.148 ms at 1080p**, **0.985 ms at 4K**, **3.775 ms for two adjacent 4K sources**, in memory-only GDI tests |
| Repeated OCR | **Two 1,000-call dense 4K traces succeeded**, with live Rust allocations and OS handles flat after warm-up |

Heap allocation and committed memory are not total physical RAM. Rendering timings measure component work, not complete UI or capture-to-paste latency. The earlier paired OCR prototype and later integrated-release timings are separate measurements. Windows cache memory still fluctuated during the repeated-use tests; full-app leak testing remains open.

### Compared with Windows Snipping Tool

**A direct comparison with Microsoft's stock Snipping Tool is still pending.** We have not measured a stock-versus-UL speed or RAM advantage. Microsoft Snipping Tool 11.2608.41.0 is now installed on the test host; matched runtime measurements have not yet been completed.

The planned matched tests cover startup, idle CPU and memory, capture-to-editor, capture-to-clipboard readiness, OCR and sustained interactive use. Both applications need the same Windows environment, authored images, display settings and comparable workflows. [Full results, methods and comparison plan](docs/PERFORMANCE.md)

**0.3.22 is a preview.** The release passed 348 automated tests, plus native OCR lifecycle checks. Broader native workflows, mixed-monitor acceptance, live image-host uploads and a full interactive leak soak remain open. [Release verification](docs/VERIFICATION-0.3.22.md)

## Build and contribute

Windows MSVC Rust toolchain, Rust 1.89 or newer:

```powershell
cargo fmt --all -- --check
cargo test --locked
./scripts/Test-CurrentUserIntegration.ps1
./scripts/Build-Portable.ps1
```

Portable builds remap machine-specific build paths. Tests use generated content; native GUI checks require isolation or explicit user authorization. Please include your Windows version, display scale and a reproducible sequence in [bug reports](https://github.com/finalpantasy/Snipping-Tool-Ultralite/issues).

This standalone project derives from MIT-licensed [HydroShot](https://github.com/Real-Fruit-Snacks/HydroShot). Required attribution is preserved in [LICENSE](LICENSE) and [third-party notices](THIRD_PARTY_NOTICES.md).
