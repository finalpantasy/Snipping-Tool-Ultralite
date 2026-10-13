//! Actual client rendering over authored backdrops; no windows or desktop access.
use snipping_tool_ul::{
    capture::CapturedScreen,
    config::Config,
    editor_ui::EditorUi,
    geometry::{Color, Point, Size},
    icons::IconCache,
    launcher::{Menu, Model, WIDTH},
    preview::EditorView,
    preview_render::PreviewRenderer,
    settings_ui::{SettingsUi, WIN_H, WIN_W},
    state::OverlayState,
    theme::{self, ThemeMode},
    tools::{Annotation, ToolKind},
    ui::Canvas,
};
use tiny_skia::Pixmap;

fn composite(p: &Pixmap) -> Pixmap {
    let mut out = p.clone();
    for (i, px) in out.data_mut().chunks_exact_mut(4).enumerate() {
        let x = (i % p.width() as usize) as f32 / p.width() as f32;
        let y = (i / p.width() as usize) as f32 / p.height() as f32;
        // Smooth colors stand for an already blurred native backdrop, never a capture.
        let blue = (1. - (x * 1.5 + y * 0.5)).clamp(0., 1.);
        let violet = (x - y * 0.45).clamp(0., 1.);
        let warm = (y * 1.3 - x * 0.2 - 0.4).clamp(0., 1.);
        let bg = [
            (44. + 42. * violet + 92. * warm) as u8,
            (62. + 72. * blue + 30. * warm) as u8,
            (88. + 77. * blue + 66. * violet) as u8,
        ];
        let inv = 255 - px[3] as u32;
        for c in 0..3 {
            px[c] = (px[c] as u32 + (bg[c] as u32 * inv + 127) / 255) as u8;
        }
        px[3] = 255;
    }
    out
}
fn contrast(a: (u8, u8, u8), b: (u8, u8, u8)) -> f64 {
    let lum = |rgb: (u8, u8, u8)| {
        let channel = |c: u8| {
            let v = c as f64 / 255.;
            if v <= 0.04045 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * channel(rgb.0) + 0.7152 * channel(rgb.1) + 0.0722 * channel(rgb.2)
    };
    let (a, b) = (lum(a), lum(b));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}
fn main() {
    let output = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "target/material-preview-frost".into());
    let out = std::path::Path::new(&output);
    std::fs::create_dir_all(out).unwrap();
    let cfg = Config::default();
    let mut doc = Pixmap::new(500, 240).unwrap();
    doc.fill(tiny_skia::Color::WHITE);
    let mut icons = IconCache::new();
    let mut c = Canvas {
        pixmap: &mut doc,
        scale: 1.,
        icons: &mut icons,
    };
    c.label(25., 20., "Captured reference", 22., (35, 36, 43));
    c.label(
        25.,
        65.,
        "A sharp screenshot inside a frosted workspace.",
        16.,
        (35, 36, 43),
    );
    c.label(
        25.,
        110.,
        "The image stays fully opaque.",
        16.,
        (35, 36, 43),
    );
    c.label(
        25.,
        195.,
        "Synthetic pixels. No desktop screenshot.",
        12.,
        (90, 95, 105),
    );
    for (mode, name) in [(ThemeMode::Dark, "dark"), (ThemeMode::Light, "light")] {
        theme::set_mode(mode);
        let tint = theme::bg_1();
        let a = theme::glass_alpha() as u32;
        let background = if mode == ThemeMode::Dark { 255 } else { 0 };
        let channel = |v: u8| ((v as u32 * a + background * (255 - a) + 127) / 255) as u8;
        let worst = (channel(tint.0), channel(tint.1), channel(tint.2));
        let ratio = contrast(theme::text_muted(), worst);
        assert!(ratio >= 4.5, "Body labels must remain readable: {ratio}");
        println!("{name}: secondary text worst-case solid backdrop contrast {ratio:.2}:1");
        let mut home = Model::new(cfg.workflow.clone(), cfg.hotkey.capture.clone());
        for (menu, label) in [
            (Menu::None, "home"),
            (Menu::Options, "home-options"),
            (Menu::UploadConfirm, "upload-confirm"),
            (Menu::Hosts, "image-hosts"),
        ] {
            home.menu = menu;
            let mut frame = Pixmap::new(WIDTH, home.height()).unwrap();
            home.render(&mut frame);
            composite(&frame)
                .save_png(out.join(format!("{name}-{label}.png")))
                .unwrap();
        }
        home.has_image = true;
        home.status = "Please wait 5s before another upload.".into();
        for (menu, label) in [
            (Menu::None, "upload-cooldown"),
            (Menu::UploadConfirm, "upload-confirm-cooldown"),
        ] {
            home.menu = menu;
            let mut frame = Pixmap::new(WIDTH, home.height()).unwrap();
            home.render(&mut frame);
            composite(&frame)
                .save_png(out.join(format!("{name}-{label}.png")))
                .unwrap();
        }
        home.has_image = false;
        home.status.clear();
        for (host, label) in [
            (snipping_tool_ul::upload::UploadHost::Freeimage, "freeimage"),
            (
                snipping_tool_ul::upload::UploadHost::Litterbox1h,
                "litterbox",
            ),
        ] {
            home.workflow.upload_host = host;
            home.menu = Menu::UploadConfirm;
            let mut frame = Pixmap::new(WIDTH, home.height()).unwrap();
            home.render(&mut frame);
            composite(&frame)
                .save_png(out.join(format!("{name}-{label}-confirm.png")))
                .unwrap();
        }
        home.workflow.upload_host = Default::default();
        home.workflow.auto_save = true;
        home.history_enabled = true;
        home.workflow.confirm_upload = true;
        home.menu = Menu::Options;
        let mut on = Pixmap::new(WIDTH, home.height()).unwrap();
        home.render(&mut on);
        composite(&on)
            .save_png(out.join(format!("{name}-options-on.png")))
            .unwrap();
        let mut state = OverlayState::new(
            CapturedScreen {
                pixels: doc.data().to_vec(),
                width: 500,
                height: 240,
                x_offset: 0,
                y_offset: 0,
                scale_factor: 1.,
            },
            &cfg,
        );
        state.preview_mode = true;
        state.active_tool = ToolKind::Highlight;
        state.annotations.push(Annotation::Highlight {
            top_left: Point::new(23., 67.),
            size: Size::new(359., 19.),
            color: Color::yellow(),
        });
        let mut ui = EditorUi::new(&cfg);
        state.active_tool = ToolKind::Select;
        state.selected_index = Some(0);
        for (w, h, dpi, label) in [
            (940, 612, 1., "select"),
            (660, 390, 1., "select-minimum"),
            (1410, 918, 1.5, "select-dpi150"),
        ] {
            let mut frame = Pixmap::new(w, h).unwrap();
            let view = EditorView::default();
            ui.draw_base(&mut frame, dpi as f32);
            PreviewRenderer::default().draw(&state, &mut frame, &view, dpi);
            ui.draw_controls(
                &mut frame,
                dpi as f32,
                &mut state,
                view.framed_layout(500, 240, w, h, dpi).scale,
            );
            composite(&frame)
                .save_png(out.join(format!("{name}-{label}.png")))
                .unwrap();
        }
        state.active_tool = ToolKind::Highlight;
        state.selected_index = None;
        for (w, h, dpi, suffix) in [
            (940, 612, 1., ""),
            (660, 390, 1., "-minimum"),
            (1410, 918, 1.5, "-dpi150"),
        ] {
            let mut frame = Pixmap::new(w, h).unwrap();
            let view = EditorView::default();
            ui.draw_base(&mut frame, dpi as f32);
            PreviewRenderer::default().draw(&state, &mut frame, &view, dpi);
            ui.draw_controls(
                &mut frame,
                dpi as f32,
                &mut state,
                view.framed_layout(500, 240, w, h, dpi).scale,
            );
            composite(&frame)
                .save_png(out.join(format!("{name}-editor{suffix}.png")))
                .unwrap();
        }
        for (action, label) in [
            (snipping_tool_ul::launcher::Action::New, "focus-new"),
            (snipping_tool_ul::launcher::Action::Button(7), "focus-tool"),
            (snipping_tool_ul::launcher::Action::ResetZoom, "focus-zoom"),
        ] {
            ui.header.keyboard_focus = true;
            ui.header.hovered = Some(action);
            let mut frame = Pixmap::new(940, 612).unwrap();
            let view = EditorView::default();
            ui.draw_base(&mut frame, 1.);
            PreviewRenderer::default().draw(&state, &mut frame, &view, 1.);
            ui.draw_controls(
                &mut frame,
                1.,
                &mut state,
                view.framed_layout(500, 240, 940, 612, 1.).scale,
            );
            composite(&frame)
                .save_png(out.join(format!("{name}-{label}.png")))
                .unwrap();
        }
        home.menu = Menu::Options;
        home.keyboard_focus = true;
        home.hovered = Some(snipping_tool_ul::launcher::Action::Quit);
        let mut focused = Pixmap::new(WIDTH * 2, home.height() * 2).unwrap();
        home.render(&mut focused);
        composite(&focused)
            .save_png(out.join(format!("{name}-options-quit-200.png")))
            .unwrap();
        let mut config = Config::default();
        config.general.theme = name.into();
        let mut settings = SettingsUi::new(config);
        settings.active_tab = 2;
        for dpi in [1., 1.5, 2.] {
            let mut frame =
                Pixmap::new((WIN_W as f32 * dpi) as u32, (WIN_H as f32 * dpi) as u32).unwrap();
            settings.draw(&mut frame);
            composite(&frame)
                .save_png(out.join(format!("{name}-settings-{}.png", (dpi * 100.) as u32)))
                .unwrap();
        }
        settings.active_tab = 0;
        settings.save_error =
            Some("Could not save settings. Check folder access and retry.".into());
        let mut error_frame = Pixmap::new(WIN_W, WIN_H).unwrap();
        settings.draw(&mut error_frame);
        composite(&error_frame)
            .save_png(out.join(format!("{name}-settings-save-error.png")))
            .unwrap();
        settings.save_error = None;
        for exit in [false, true] {
            settings.config.general.quit_on_close = exit;
            let mut frame = Pixmap::new(WIN_W, WIN_H).unwrap();
            settings.draw(&mut frame);
            composite(&frame)
                .save_png(out.join(format!(
                    "{name}-settings-close-{}.png",
                    if exit { "quit" } else { "tray" }
                )))
                .unwrap();
        }
    }
    theme::set_mode(ThemeMode::Dark);
    println!("Actual light/dark clients rendered headlessly over an illustrative backdrop.");
}
