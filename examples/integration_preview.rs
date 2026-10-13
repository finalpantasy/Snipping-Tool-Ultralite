//! Synthetic settings-tab evidence. No HWND, installation, clipboard or capture.
fn main() {
    let out = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "target/integration-preview.png".into());
    let mut p = tiny_skia::Pixmap::new(420, snipping_tool_ul::settings_ui::WIN_H).unwrap();
    p.fill(tiny_skia::Color::from_rgba8(28, 28, 33, 255));
    snipping_tool_ul::windows_integration::draw(&mut p, false, "", None);
    let mut icons = snipping_tool_ul::icons::IconCache::new();
    let mut c = snipping_tool_ul::ui::Canvas {
        pixmap: &mut p,
        scale: 1.,
        icons: &mut icons,
    };
    c.label(
        24.,
        20.,
        "Snipping Tool UL Settings",
        16.,
        snipping_tool_ul::ui::INK,
    );
    for (i, name) in ["General", "Shortcuts", "Toolbar", "Windows"]
        .iter()
        .enumerate()
    {
        c.label(
            36. + i as f32 * 93.,
            58.,
            name,
            14.,
            if i == 3 {
                snipping_tool_ul::ui::focus_color()
            } else {
                snipping_tool_ul::ui::MUTED
            },
        );
    }
    c.button(
        snipping_tool_ul::ui::Rect::new(
            140.,
            snipping_tool_ul::settings_ui::WIN_H as f32 - 54.,
            140.,
            34.,
        ),
        "Save & Close",
        None,
        false,
        false,
        true,
    );
    p.save_png(out).unwrap();
}
