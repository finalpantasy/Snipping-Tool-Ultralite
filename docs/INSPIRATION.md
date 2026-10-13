# Source research

Source inspected on 2026-10-12. Other projects are references, not dependencies. Publisher performance claims are not comparable measurements on this machine.

* Crisp (MIT): native Win32 drawing, clipboard DIB+PNG prepared before opening clipboard, ten bounded retries. Our arboard backend already provides PNG and DIBV5 with bounded busy retries; preserve it and add stale-write protection. Magnifier excluded. Scrolling capture deferred.
* winshotx (MIT): separates fast/live selection from freezing screen images; documents avoiding large raw-image IPC and using PNG for browser presentation. Keep our native buffers local with no browser IPC; retain one image authority for preview/export.
* MiniSnip (MIT): clips virtual-desktop rectangles and normalizes negative origins. Our ScreenRect already provides tested physical-coordinate normalization. Preserve it.
* Nano Snipper: workspace manifests declare MIT but a complete root license notice was not found in this review. Inspect architecture ideas only: cached GPU resources and independent acquire/readback/present timing. Add independent capture/clipboard timing; do not add a permanent desktop texture cache without measured benefit.
* Greenshot and ShareX (GPL-3.0): destination workflows, uploader extensibility, clipboard formats, robust capture backends, and editor affordances. Use workflow ideas; no code transplanted into this MIT source base.
* HydroShot (MIT): retained region capture, annotations, undo/redo, OCR, pinning, timer, explicit upload, and history. Required credits retained; capture memory and DPI issues addressed in our development baseline.

Sources: https://github.com/shadesofdeath/Crisp , https://github.com/Mun1to/winshotx , https://github.com/Mirochill/MiniSnip , https://github.com/bitcoin-verse/nano-snipper , https://github.com/greenshot/greenshot , https://github.com/ShareX/ShareX , https://github.com/Real-Fruit-Snacks/HydroShot . Read-only pinned research artifacts remain outside this repository; no borrowed application was executed during this review.
