//! Actual About drawing code, synthetic output only; no windows or side effects.
use snipping_tool_ul::{
    about,
    theme::{self, ThemeMode},
};
fn main() {
    let output = std::path::PathBuf::from(std::env::args().nth(1).expect("Output directory"));
    std::fs::create_dir_all(&output).unwrap();
    for (name, mode) in [("dark", ThemeMode::Dark), ("light", ThemeMode::Light)] {
        theme::set_mode(mode);
        for dpi in [1., 1.25, 1.5, 2.] {
            let mut frame = tiny_skia::Pixmap::new(
                (about::WIDTH as f32 * dpi) as u32,
                (about::HEIGHT as f32 * dpi) as u32,
            )
            .unwrap();
            about::draw(&mut frame, false, false);
            frame
                .save_png(output.join(format!("{name}-about-{}.png", (dpi * 100.) as u32)))
                .unwrap();
        }
    }
}
