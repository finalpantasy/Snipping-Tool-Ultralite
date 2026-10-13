# 0.3.21 upload feedback

Upload now acknowledges the click with a preparation state before synchronous image flattening. The button shows Preparing, Uploading or Copying, then Link copied, Copy link or Retry. A larger card is aligned directly under Upload in both the editor and compact home window. It shows the URL and distinguishes successful upload from successful clipboard copy, including a newer clipboard value being preserved.

The card respects open-menu Escape handling and can be dismissed with its close control or Escape. Subsequent editing/navigation clears completed feedback. Its hit targets consume clicks instead of passing through to the image. Starting a new capture clears old feedback; stale completions cannot show a prior image's success on a new one. Existing confirmation, five-second cooldown, single request and cached-link reuse rules are retained.

## Headless verification

- 328 locked offline all-target tests passed; 0 failed, 3 manual tests ignored.
- Typed success/copy-failure/newer-clipboard outcomes, progress truthfulness, card alignment and bounds, click interception, and screenshot-export separation are covered.
- Actual app-code feedback renders inspected in light/dark themes, minimum editor size, 150% DPI and compact home. No native windows or actual image uploads were used.
- Formatting, whitespace, mocked installer, remapped build, public-identity scans, package entry hashes and quiet installed executable/metadata/notices/Start target read-back.

No new dependency, animation, permanent render loop, network request type or image buffer was added. The preparation acknowledgement adds one presentation before the existing preparation work. Native perceived latency, clipboard access, live provider behavior and full interaction remain isolated-session acceptance gaps. The same verified executable is distributed in the 0.3.21 GitHub preview. Publication updates the package documentation and labelled light gallery without changing app code.
