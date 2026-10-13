# GitHub publication checklist

Prepared destination: `finalpantasy/Snipping-Tool-UL`, public standalone repository. Product: **Snipping Tool UL**. Description: **Ultralight native Rust Snipping Tool replacement for Windows 11: instant clipboard captures, annotations, hotkeys, timers and one-click image sharing.**

Suggested topics: `windows`, `windows-11`, `snipping-tool`, `screenshot`, `rust`, `clipboard`, `annotation`, `lightweight`, `image-sharing`.

Publish as a preview; no verified end-to-end speed comparison, total-app memory claim or universal Reddit compatibility claim. Preserve upstream MIT and dependency notices. Use the account handle and GitHub noreply identity for commit authors/committers. Scan the public files, package, diff and commit metadata before every push.

## Evidence still needed

- Real native working screenshot or GIF in README. The existing client illustrations are labeled synthetic, and cannot satisfy this requirement. Use an isolated Windows VM/session, or a user-supplied current screenshot of the app working with non-sensitive content.
- Isolated native workflow checks: image paste, capture modes across mixed-DPI monitors, timer/hotkey interaction, uploader success/failure/retry, shell appearance, close-to-tray/full-exit behavior and total-app memory/latency. Do not run these on the user's working desktop.

Source can be published as a preview with clearly documented native acceptance gaps once the real-media requirement is met. A stable release or performance promise needs the native workflow evidence. Prepared publication metadata does not create a repository, push, tag or release.
