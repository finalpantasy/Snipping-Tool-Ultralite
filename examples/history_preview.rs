//! Production history empty states; authored pixels, no windows or storage access.
use snipping_tool_ul::{
    history_ui::{draw_empty_history, WIN_H, WIN_W},
    theme::{self, ThemeMode},
};
fn main() {
    let out = std::path::PathBuf::from(std::env::args().nth(1).expect("output directory"));
    std::fs::create_dir_all(&out).unwrap();
    for (name, mode) in [("dark", ThemeMode::Dark), ("light", ThemeMode::Light)] {
        theme::set_mode(mode);
        for dpi in [1., 1.25, 1.5, 2.] {
            for enabled in [false, true] {
                let mut p = tiny_skia::Pixmap::new(
                    (WIN_W as f32 * dpi) as u32,
                    (WIN_H as f32 * dpi) as u32,
                )
                .unwrap();
                draw_empty_history(&mut p, enabled);
                p.save_png(out.join(format!(
                    "{name}-history-{}-dpi{}.png",
                    if enabled { "empty" } else { "off" },
                    (dpi * 100.) as u32
                )))
                .unwrap();
            }
        }
    }
}
