<p align="center"><img src="assets/ul-icon.png" width="88" alt="Snipping Tool UL logo"></p>

<h1 align="center">Snipping Tool UL</h1>
<p align="center">A fast, lightweight snipping tool for Windows 11, written in native Rust.</p>

<p align="center"><a href="https://github.com/finalpantasy/Snipping-Tool-Ultralite/releases/download/v0.3.23-preview/Snipping-Tool-UL-0.3.23-Windows-x64.zip"><strong>Download 0.3.23 preview</strong></a> · <a href="docs/USAGE.md">User guide</a> · <a href="LICENSE">MIT license</a></p>

![Snipping Tool UL editor in light mode](docs/media/editor-light.png)

Take a snip and it is already on your clipboard, with the editor open. Annotate it, then copy, save, upload or pin it.

## Compared with Windows Snipping Tool

Same PC and Windows session, three 4K monitors at 150% scaling. 30 runs of each app after 5 warm-up runs; medians. Measured October 14, 2026 with UL 0.3.22 and Microsoft Snipping Tool 11.2608.41.0.

**Taking a snip** (hotkey, drag a 1280 x 720 region, release)

| | Snipping Tool UL | Windows Snipping Tool |
| --- | ---: | ---: |
| Hotkey to selection screen | **155 ms** | 355 ms |
| Release to image on clipboard | **50 ms** | 76 ms |
| Release to editor open | **78 ms** | no editor, shows a notification |
| Memory while capturing | **29 MB** | 484 MB |

**Opening the app**

| | Snipping Tool UL | Windows Snipping Tool |
| --- | ---: | ---: |
| Launch to window | **47 ms** | 235 ms |
| CPU time spent starting | **94 ms** | 469 ms |
| Memory, window open | **10 MB** | 102 MB |
| Threads | **19** | 99 |
| CPU while idle | none measurable | none measurable |
| Size on disk | **10 MB**, one exe | about 420 MB, plus Windows App Runtime |

Memory is private bytes. 59 of 60 measured snips were exact; one UL snip came out larger than the drawn area. UL stays in the tray between snips (about 43 MB after a capture); Windows Snipping Tool exits after each snip. OCR speed has not been compared yet. [Method and limits](docs/PERFORMANCE.md)

## What it does

| Capture | Edit | Share |
| --- | --- | --- |
| Region, window or full screen | Pen, arrows, shapes, highlighter | Copied to the clipboard on capture |
| Screen frozen while you select | Text, numbered steps, pixelate | Save at full resolution |
| 3, 5 or 10 second timer | Undo, redo, zoom, pan | One-click public image link |
| Custom hotkeys | Canvas grows when you draw past the edge | Pin on top of other windows |
| Light and dark mode | Copy text with local OCR | Optional history and auto-save |

Uploads go to Catbox, Freeimage.host or Litterbox (temporary), with no account needed. Links are public. [Hosting details](docs/UPLOAD-HOSTS.md)

<details>
<summary>More screenshots</summary>

![Compact home window](docs/media/home-light.png)

![Upload progress and copied link](docs/media/upload-light.png)

</details>

## Install

1. [Download the ZIP](https://github.com/finalpantasy/Snipping-Tool-Ultralite/releases/download/v0.3.23-preview/Snipping-Tool-UL-0.3.23-Windows-x64.zip) and extract it.
2. Run `snipping-tool-ul.exe`.
3. Click **New** or press **Ctrl+Shift+S**.

It runs portable, with no installer. History and auto-save are off until you turn them on. **Options > Quit Snipping Tool UL** exits completely.

**Settings > Windows > Use as my Snipping Tool** installs UL for your account. If you also tick the replacement box, it **uninstalls Microsoft's Snipping Tool** for your account and names UL "Snipping Tool" in Start. You can reinstall Microsoft's app from the [Microsoft Store](https://apps.microsoft.com/detail/9mz95kl8mr0l). Win+Shift+S is not remapped to UL.

## Status

0.3.23 is a preview. 358 automated tests pass. Not yet verified: mixed-scale multi-monitor setups, live uploads, and memory over long sessions. [Changelog](CHANGELOG.md) · [Release checks](docs/VERIFICATION-0.3.23.md)

## Build

Windows MSVC Rust toolchain, Rust 1.89 or newer:

```powershell
cargo fmt --all -- --check
cargo test --locked
./scripts/Test-CurrentUserIntegration.ps1
./scripts/Build-Portable.ps1
```

Bug reports are welcome in [Issues](https://github.com/finalpantasy/Snipping-Tool-Ultralite/issues). Please include your Windows version, display scale and the steps to reproduce.

Snipping Tool UL is based on the MIT-licensed [HydroShot](https://github.com/Real-Fruit-Snacks/HydroShot). Attribution is kept in [LICENSE](LICENSE) and [third-party notices](THIRD_PARTY_NOTICES.md).
