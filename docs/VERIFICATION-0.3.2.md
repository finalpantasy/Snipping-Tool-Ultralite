# 0.3.2 teal accents

Primary button and slider fill is #007c91, inspired by the teal scissors reference. White #f9fbf8 labels have 4.70:1 contrast. Selected text, active outlines and keyboard focus use lighter aqua #53c8d6 in dark mode and deeper teal #006a7c in light mode. Enabled settings controls use the same readable label color. Annotation color presets and shell icon assets are unchanged; the reference logo remains tentative.

The frosted surfaces, dimensions, capture/export behavior and event-driven rendering are preserved. No assets, dependencies, image decoding or animation work were added.

175 Rust tests, formatting and five mocked installer scenarios pass. Inspected actual headless client renders in both themes, default/minimum sizes, 150 percent editor DPI and 100/150/200 percent settings DPI. All renders use synthetic content over an authored backdrop. They create no windows and perform no capture, clipboard write or upload. Portable packaging checks safe CLI commands, public identity scans, archive entries and hashes. Versioned installation preserves running processes; Quit from the tray and reopen to load the update.

Native workflow acceptance and GitHub publication remain pending as documented in [0.3.0 verification](VERIFICATION-0.3.0.md). This color change does not establish new end-to-end latency or memory results.
