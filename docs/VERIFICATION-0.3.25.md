# Snipping Tool UL 0.3.25: Ko-fi link in About

About now shows a **Support on Ko-fi** button beside Close. It opens only `https://ko-fi.com/finalpantasy` in the default browser and does nothing else on the network.

## Verification

- All automated tests pass, including a new test for the About button row: both buttons share one row, a 12 px gap, the row is centred, and the address is exact.
- The About window was rendered headlessly in light and dark at 100%, 125%, 150% and 200% scaling and inspected.
- Code changes since 0.3.24 are limited to the About window, one added icon and their previews and tests; capture, clipboard and editor code are unchanged, so the 0.3.24 native snip results still apply. No new live-desktop snip test was run for this release.
- Remapped Windows build, formatting, source and package identity scans and archive-entry hashes checked before publication.
