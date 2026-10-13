<p align="center"><img src="assets/ul-icon.png" width="88" alt="Snipping Tool UL teal bird and scissors logo"></p>

<h1 align="center">Snipping Tool UL</h1>
<p align="center"><strong>Ultralight in performance, not features.</strong><br>A lightweight native Rust snipping tool for Windows 11.</p>

<p align="center"><a href="https://github.com/finalpantasy/Snipping-Tool-UL/releases/download/v0.3.15-preview/Snipping-Tool-UL-0.3.15-Windows-x64.zip"><strong>Download Windows preview</strong></a> · <a href="https://finalpantasy.github.io/Snipping-Tool-UL/">Product page & video</a> · <a href="docs/USAGE.md">User guide</a> · <a href="LICENSE">MIT license</a></p>

![The real UL editor on Windows 11, showing a fictional captured project brief](docs/media/editor-dark.png)

Take a snip. It is copied immediately, and the preview opens automatically. Paste it into a conversation or keep editing without another button click.

**Native Rust. No webview. No permanent render loop.** Teal frosted controls, a themed title bar, and dark mode by default.

## See it

[![Watch the 32-second native UI product tour](docs/media/demo-poster.png)](https://finalpantasy.github.io/Snipping-Tool-UL/#demo)

[Watch the tour](https://finalpantasy.github.io/Snipping-Tool-UL/#demo) · [Download MP4](docs/media/Snipping-Tool-UL-demo.mp4)

Screenshots and tour use the real Windows app with a fictional sample brief. The tour uses captions, crops and cuts to show the interface; it does not simulate UI actions. Capture, automatic clipboard copy and wheel zoom were exercised during recording. See [media provenance](docs/MEDIA.md).

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

![Real UL home window with New, capture mode, Delay and Options](docs/media/home-dark.png)

</details>

## Start using it

1. [Download the Windows x64 ZIP](https://github.com/finalpantasy/Snipping-Tool-UL/releases/download/v0.3.15-preview/Snipping-Tool-UL-0.3.15-Windows-x64.zip) and extract it.
2. Launch `snipping-tool-ul.exe`.
3. Choose **New** or press **Ctrl+Shift+S**.

Portable use needs no installer. **Options > Quit Snipping Tool UL** exits completely. Choose **Keep in tray** or **Quit completely** for the X button in Settings. History and auto-save are off by default.

Optional **Settings > Windows > Use as my Snipping Tool** installs for your account and can replace Microsoft's Snipping Tool entry in Start. The installer explains and confirms the replacement. Windows' reserved Win+Shift+S shortcut is not remapped. [Setup and usage guide](docs/USAGE.md)

## Small by design, measured honestly

History image copying now keeps one active preparation and only the newest pending request. Generated-image measurements reduced requested Rust heap peak from **63.28 to 31.92 MiB at 4K**, and **253.13 to 126.84 MiB at 8K**, while keeping original pixels.

Two 1,000-call native 4K OCR traces kept live Rust allocations and OS handles flat after warm-up. Windows cache memory still fluctuates. These are component measurements, not total app RAM or a speed comparison against Windows Snipping Tool. [Memory investigation](docs/LEAK-INVESTIGATION-2026-10-13.md)

**0.3.15 is a preview.** The release passed 321 automated tests, plus native OCR lifecycle checks. Broader native workflows, mixed-monitor acceptance, live image-host uploads and a full interactive leak soak remain open. [Release verification](docs/VERIFICATION-0.3.15.md)

## Build and contribute

Windows MSVC Rust toolchain, Rust 1.89 or newer:

```powershell
cargo fmt --all -- --check
cargo test --locked
./scripts/Test-CurrentUserIntegration.ps1
./scripts/Build-Portable.ps1
```

Portable builds remap machine-specific build paths. Tests use generated content; native GUI checks require isolation or explicit user authorization. Please include your Windows version, display scale and a reproducible sequence in [bug reports](https://github.com/finalpantasy/Snipping-Tool-UL/issues).

This standalone project derives from MIT-licensed [HydroShot](https://github.com/Real-Fruit-Snacks/HydroShot). Required attribution is preserved in [LICENSE](LICENSE) and [third-party notices](THIRD_PARTY_NOTICES.md).
