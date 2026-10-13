# Snipping Tool UL 0.3.15 preview

This preview includes the History copy and OCR lifecycle fixes documented in the [memory investigation](LEAK-INVESTIGATION-2026-10-13.md). The release changes the version and installer default from 0.3.14 to 0.3.15; no additional application runtime features were added for recording or publication.

## Verified

- 321 locked, offline all-target automated tests passed, zero failures; three manual diagnostics remained ignored.
- A portable Windows x64 release was built with machine-specific user and workspace paths remapped.
- The installer integration script passed its mocked scenarios, without modifying real Start folders or Appx registration.
- The application runtime files match the independently reviewed leak-fix source. Native OCR repeat, colour, cancellation, timeout and shutdown checks and two 1,000-call dense 4K resource traces remain applicable. Component live Rust allocations and handles stayed flat; OS cache memory still fluctuated.
- During the user-authorized recording, the real native app opened, captured fictional sample content, reported automatic clipboard copy, accepted the capture hotkey, and performed pointer-centered wheel zoom. The source app was closed normally afterward. Its recording configuration was isolated from the installed app's settings.
- Public media is limited to the native app window and authored sample document. Interruption takes and later raw footage are excluded. The captioned product tour does not simulate UI actions. See [media provenance](MEDIA.md).
- Repository files, release package, commit identity and media metadata are scanned before publication. Release assets use the account handle and contain no personal identity or user-directory paths.

## Boundaries

The media recording is a bounded native check, not a full acceptance suite. Mixed-monitor capture acceptance, annotation interactions under repeated native use, paste into external applications, real image-host success and failure cases, timers, tray behavior, full-app memory and a long interactive leak soak remain open. Do not treat component measurements as total app RAM or a comparative speed claim against Windows Snipping Tool.

The installed 0.3.14 files remain unchanged. This preview is a portable download; publishing it does not install it or change Windows integration. User-authorized temporary recording windows were cleared when the user resumed work, and remaining publication checks ran in the background.
