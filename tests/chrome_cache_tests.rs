//! Chrome cache parity. Changes the global theme, so it runs in its own process.
#[test]
fn chrome_cache_replays_byte_identical_frames_from_a_few_rows() {
    use snipping_tool_ul::{
        config::Config,
        editor_ui::EditorUi,
        theme::{self, ThemeMode},
    };
    for mode in [ThemeMode::Light, ThemeMode::Dark] {
        theme::set_mode(mode);
        for (w, h, dpi) in [
            (940, 612, 1.),
            (1410, 918, 1.5),
            (1320, 780, 2.),
            (661, 391, 1.25),
        ] {
            let ui = EditorUi::new(&Config::default());
            let mut fresh = tiny_skia::Pixmap::new(w, h).unwrap();
            ui.draw_base(&mut fresh, dpi);
            // A replay over stale content must reproduce the first render.
            let mut replay = tiny_skia::Pixmap::new(w, h).unwrap();
            replay.fill(tiny_skia::Color::from_rgba8(200, 10, 90, 255));
            ui.draw_base(&mut replay, dpi);
            assert!(fresh.data() == replay.data(), "{mode:?} {w}x{h}@{dpi}");
            // A full-frame copy would hold w * h * 4 bytes.
            assert!(ui.frame_cache_bytes() * 10 < (w * h * 4) as usize);
        }
    }
}
