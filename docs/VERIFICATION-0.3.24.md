# Snipping Tool UL 0.3.24: polish, update check and faster copy

Six bug fixes, a measured alignment pass across every window, less editor memory, a weekly update check that can be turned off, and clipboard copies that never leave the clipboard locked or empty.

## Changes

- Bugs: keyboard minimize with the annotation menu open could crash the editor; right-click during a text edit could act on the wrong annotation; drags to the right or bottom screen edge lost the last pixel row or column; the shortcut recorder refused Ctrl+Shift+digit and AZERTY digits; switching tools mid-drag left a ghost shape and the eyedropper sampled outside the image; a failed history copy gave no message.
- Alignment: controls that were 1 to 8 px off centre or off baseline in Home, the editor, Settings, the host menu, the upload notice and History were measured at 100% and 150%, light and dark, and corrected. [Before and after](images/alignment-before-after-light.png), [pass notes](POLISH-PASS-2026-10-14.md).
- Memory: the open editor holds about 5 MB less at the default size (33 MB less at 4K); Copy, Save, Upload and Pin without annotations use half the peak memory.
- Update check: at most once every 7 days, at start, one request to the GitHub releases API. It only shows an "Update available" link and never downloads anything. Off switch: Settings > General.
- Clipboard: the image is fully prepared before the clipboard is opened; a 4K copy prepares about 20% faster and uses about 33 MB less temporary memory.

## Verification

- All locked all-target automated tests pass after merging the three branches, including new tests for each fix, version comparison, the 7-day gate and exact clipboard bytes.
- Native region capture with the 0.3.24 build on Windows 11 (three 4K monitors at 150%, against a known tile pattern, no other input): normal drags 10 of 10 exact; release-then-flick 29 of 30 exact. The one miss produced no clipboard image within 4 s; 20 further flick trials with the app log captured were all exact and logged no clipboard errors, so its cause is not confirmed.
- Remapped Windows build, formatting, source and package identity scans and archive-entry hashes checked before publication.

Not yet verified in the running app: the update link appearing, pasting into Word, Paint and browsers, and memory after the editor closes.
