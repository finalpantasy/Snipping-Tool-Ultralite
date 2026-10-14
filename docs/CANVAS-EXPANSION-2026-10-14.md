# Automatic annotation canvas

The subsequent [canvas bug test](CANVAS-BUG-TEST-2026-10-14.md) supersedes the initial candidate below: four corrections, 348 passing tests and a new retained executable. Initial evidence is preserved for comparison.

Local source candidate, 2026-10-14. Draw outside any image edge to extend the canvas with white space. Moving, resizing, changing stroke size or deleting annotations recomputes the required space, never shrinking below the original image. Undo and redo derive the same bounds from restored marks.

The source image and pointer transform stay fixed during drawing. Middle-mouse pan and wheel zoom reach further into the workspace. The dimension label reports the live canvas, including text being typed. Copy, Save, Upload and Pin use the same full-resolution canvas. Original capture pixels and raw OCR input stay at their original dimensions. Pixelation remains restricted to the original image; it does not sample added white space.

Verification:

- 340 locked, offline, all-target automated tests passed; zero failures, three existing manual tests ignored. Twelve focused canvas tests cover all four stroked edges, live drawing, text, shrink, move, deletion, size controls, undo/redo, remaining marks, alpha, pixelation, pan/zoom/DPI, caching and allocation bounds. An unclipped raster reference checks every annotation tool's exported ink bounds, including marker borders and measurement ticks.
- Formatting and diff whitespace checks passed.
- Remapped release build completed without compiler warnings. Retained candidate: `target/canvas-candidate-20261014/snipping-tool-ul.exe`, SHA-256 `DC8C8F71435B6F295FB8D326DEB2866FCADCC111ED1C5E8195863EE385605C66`. CLI version check exited successfully; ASCII/UTF-16 personal-identity scan returned zero matches. The local candidate retains the source's 0.3.21 version number and differs from the published 0.3.21 executable.
- Twenty production-code editor renders and one full-resolution export were generated without native windows. Inspected live overflow, all-edge expansion, resized light-mode 150% canvas, deletion and exported borders. Evidence: `target/canvas-preview/`; test log: `target/canvas-tests.log`.
- Preview allocations stay bounded by the viewport. Full-resolution exports reject unrepresentable or oversized canvas dimensions before allocation.

Native mouse, window and clipboard workflows remain pending an isolated Windows session. This candidate has not been installed or published. The public README caption cleanup was separately authorized and published as commit `2b5a33057d0e77ff871ea345a18659866a3e1741`; that commit changes only README wording.
