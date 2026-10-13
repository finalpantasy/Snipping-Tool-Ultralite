# Audit coverage and remaining acceptance work

This pass combines source review of the changed paths, automated logic/rendering checks, synthetic performance measurements and a guarded private-desktop presentation probe. It does not certify every native workflow.

| Area | Evidence and result | Remaining native checks |
| --- | --- | --- |
| Capture, timers, New | Geometry, selection, release ordering and deadline tests; clipboard ticket reservation reviewed | Real mixed-DPI/negative-origin monitors, timer cancellation, repeated New, window enumeration |
| Clipboard | Queue/ordering and export tests; cancelled capture reservation fixed | Paste into browsers, Reddit/chat clients and desktop apps; clipboard contention |
| Editing | Tools, undo/redo, resizing, selection, text, arrows, pan/zoom and exact cached rendering tests | Human input feel and actual monitor transitions |
| History | Isolated storage/pruning, multi-format/concurrent writes and DPI tests; worker lifetime reviewed | Loading a large real gallery and interacting during disk contention |
| Settings/Options | Save failure/retry, hit targets, temporary config tests; rendered off/on badge and error message inspected | Native folder chooser latency, startup permissions and hotkey collision behavior |
| Sharing | Offline host-response, URL, confirmation, cooldown, deduplication and expiry tests; timeouts reviewed | Live provider upload, outage and Reddit/RES viewing checks; no images uploaded in this pass |
| Windows integration | Five mocked installer scenarios, remapped binary/icon/package checks and quiet installed-file verification | Genuine native working screenshots/GIF for README and isolated complete workflow acceptance |
| Rendering/performance | Before/after CPU fixtures and ten private synthetic presentation cycles | Full input-to-screen latency, steady-state app memory, leak soak and fair comparison with Microsoft Snipping Tool |
| Dependencies/identity | Fresh OSV query, Windows notice coverage and source/package identity scans | Font-library maintenance migrations; minimum Rust toolchain and other platforms |

## Remaining optimization candidates

- Native glass presentation retains a full-frame BGRA conversion buffer and presents complete frames. Removing or partially updating it needs visual validation on an isolated Windows session.
- Clipboard image encoding still precedes the ready event; preserve clipboard compatibility before changing that path.
- Pin encoding now avoids a full pixel copy but remains synchronous. Background file preparation needs defined drag/reveal behavior while encoding completes.
- An already-open History gallery refreshes when reopened through Options, rather than following new captures automatically.
- The unused legacy Imgur helper is outside the no-setup UI; its response handling should be hardened or the compatibility path removed before exposing it again.
- Settings startup-registration errors can occur after other settings have already saved. The error makes this partial outcome explicit; there is no cross-file/registry transaction.

One-pixel captures remain permitted for precision work. No automatic capture-size threshold was added. All remaining native tests must use an isolated Windows VM/session; the limited private presentation probe cannot validate capture, clipboard, global hotkeys or user-facing native appearance. No working-desktop screenshots or synthetic uploads were taken.
