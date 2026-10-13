//! Actual settings renderer with synthetic configuration, no HWND or dialogs.
use snipping_tool_ul::{
    config::Config,
    settings_ui::{SettingsUi, WIN_H, WIN_W},
};
fn main() {
    let out = std::path::Path::new("target/organization-preview");
    std::fs::create_dir_all(out).unwrap();
    let mut config = Config::default();
    config.general.save_directory = "Pictures / Snipping Tool UL".into();
    let mut ui = SettingsUi::new(config);
    for (tab, name) in [
        (0, "general"),
        (1, "shortcuts"),
        (2, "tools"),
        (3, "windows"),
    ] {
        ui.active_tab = tab;
        for scale in [1., 1.5, 2.] {
            let mut frame = tiny_skia::Pixmap::new(
                (WIN_W as f32 * scale) as u32,
                (WIN_H as f32 * scale) as u32,
            )
            .unwrap();
            ui.draw(&mut frame);
            frame
                .save_png(out.join(format!("settings-{name}-{}.png", (scale * 100.) as u32)))
                .unwrap();
        }
    }
    println!("All settings tabs rendered at 100, 150 and 200 percent without a GUI.");
}
