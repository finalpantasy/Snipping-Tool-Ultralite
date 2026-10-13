# Authored bird/scissors resources

- `ul-app.svg`: supplied ivory-tile app logo.
- `ul-logo.svg`: supplied transparent teal bird mark.
- `ul-tray.svg`: supplied single-color tray silhouette.
- `ul-icon-tiled-original.ico`: preserved original nine-size tiled icon.
- `ul-icon.ico`: transparent faceted bird, rendered offline at nine Windows sizes from `ul-logo.svg`.
- `ul-icon.png`: transparent 256px bird PNG.
- `ul-icon.rgba`: prepared 64px bird, cached for native windows and the taskbar.
- `ul-tray-on-dark.rgba` / `ul-tray-on-light.rgba`: prepared 32px silhouette pixels in bright aqua / deeper teal for shell contrast.

Regenerate into a separate directory with `cargo run --example logo_assets -- target/logo-assets`. Check the generated small-icon proof before copying the prepared files into assets. The helper preserves differing outputs. All runtime buffers use straight RGBA alpha, with no decoding or SVG parsing during icon creation.

Original ICO SHA256: `DA7F42A7E701842F980F864B218CCB605C81DC3144749297DE685A1EAD12F2A4`.
