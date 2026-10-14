//! Measures control alignment in headless renders. Never creates a window,
//! captures a desktop or touches the clipboard.
//!
//! For each control it reports, in physical pixels, how far the content block,
//! the icon and the label's cap band (cap top to baseline) sit from the
//! control's centre, plus the label's baseline row so neighbours can be
//! compared. Usage: `cargo run --example alignment_probe -- <output dir>`
use snipping_tool_ul::{
    about,
    config::Config,
    editor_ui::EditorUi,
    icons::IconCache,
    launcher::{Action, Menu, Model},
    settings_ui::{self, SettingsUi},
    theme::{self, ThemeMode},
    ui::{Canvas, Rect},
};
use std::{fmt::Write as _, path::Path, path::PathBuf};
use tiny_skia::Pixmap;

type Box4 = (i32, i32, i32, i32);

/// Pixels that differ from the control background sampled at its left edge.
fn ink(p: &Pixmap, r: Rect, s: f32) -> (Vec<bool>, i32, i32, i32, i32) {
    // Symmetric insets skip anti-aliased rounded corners and edges.
    let (x0, y0) = (
        ((r.x + 3.) * s).round() as i32,
        ((r.y + 2.) * s).round() as i32,
    );
    let (x1, y1) = (
        ((r.x + r.w - 3.) * s).round() as i32,
        ((r.y + r.h - 2.) * s).round() as i32,
    );
    let w = p.width() as i32;
    let px = |x: i32, y: i32| {
        let i = ((y * w + x) * 4) as usize;
        let d = p.data();
        [d[i] as i32, d[i + 1] as i32, d[i + 2] as i32]
    };
    let bg = px(x0 + (2. * s) as i32, (y0 + y1) / 2);
    let mut m = vec![false; ((x1 - x0) * (y1 - y0)) as usize];
    for y in y0..y1 {
        for x in x0..x1 {
            let c = px(x, y);
            let diff = (0..3).map(|k| (c[k] - bg[k]).abs()).max().unwrap();
            m[((y - y0) * (x1 - x0) + (x - x0)) as usize] = diff > 48;
        }
    }
    (m, x0, y0, x1 - x0, y1 - y0)
}

/// Column groups separated by at least `gap` empty columns, within `cols`.
fn groups(m: &[bool], w: i32, h: i32, cols: (i32, i32), gap: i32) -> Vec<Box4> {
    let mut out: Vec<Box4> = Vec::new();
    let mut empty = gap;
    for x in cols.0..cols.1 {
        let rows: Vec<i32> = (0..h).filter(|&y| m[(y * w + x) as usize]).collect();
        let (Some(&a), Some(&b)) = (rows.first(), rows.last()) else {
            empty += 1;
            continue;
        };
        if empty >= gap || out.is_empty() {
            out.push((x, a, x + 1, b + 1));
        } else {
            let g = out.last_mut().unwrap();
            g.1 = g.1.min(a);
            g.2 = x + 1;
            g.3 = g.3.max(b + 1);
        }
        empty = 0;
    }
    out
}

/// Reports one control. `icon` says whether its first group is an icon.
fn control(out: &mut String, p: &Pixmap, s: f32, name: &str, r: Rect, icon: bool) {
    let (m, ox, oy, w, h) = ink(p, r, s);
    let cx = ox as f32 + w as f32 / 2.;
    let cy = oy as f32 + h as f32 / 2.;
    let all = groups(&m, w, h, (0, w), (5. * s) as i32);
    if all.is_empty() {
        let _ = writeln!(out, "{name:<26} (no ink)");
        return;
    }
    let (left, right) = (all[0].0, all.last().unwrap().2);
    let top = all.iter().map(|g| g.1).min().unwrap();
    let bottom = all.iter().map(|g| g.3).max().unwrap();
    let mut line = format!(
        "{name:<26} block dx {:+5.1} dy {:+5.1}",
        ox as f32 + (left + right) as f32 / 2. - cx,
        oy as f32 + (top + bottom) as f32 / 2. - cy
    );
    let mut rest = all.as_slice();
    if icon {
        let g = rest[0];
        let _ = write!(
            line,
            " | icon dx {:+5.1} dy {:+5.1}",
            ox as f32 + (g.0 + g.2) as f32 / 2. - cx,
            oy as f32 + (g.1 + g.3) as f32 / 2. - cy
        );
        rest = &rest[1..];
    }
    if let (Some(first), Some(last)) = (rest.first(), rest.last()) {
        // Cap top comes from the first (capital) letter; the baseline is the
        // most common bottom row, which ignores descenders.
        let letter = groups(&m, w, h, (first.0, first.2), 1)[0];
        let mut bottoms = std::collections::HashMap::new();
        for x in first.0..last.2 {
            if let Some(y) = (0..h).rev().find(|&y| m[(y * w + x) as usize]) {
                *bottoms.entry(y + 1).or_insert(0) += 1;
            }
        }
        let base = bottoms.into_iter().max_by_key(|&(y, n)| (n, -y)).unwrap().0;
        let _ = write!(
            line,
            " | caps dy {:+5.1} baseline {:>4}",
            oy as f32 + (letter.1 + base) as f32 / 2. - cy,
            oy + base
        );
    }
    let _ = writeln!(out, "{line}");
}

fn save(p: &Pixmap, dir: &Path, name: &str, mode: &str, s: f32) {
    p.save_png(dir.join(format!("{name}-{mode}-{}.png", (s * 100.) as u32)))
        .unwrap();
}

fn launcher(out: &mut String, dir: &Path, mode: &str, s: f32) {
    let w = 660.;
    let mut model = Model::new(Default::default(), "Ctrl+Shift+S".into());
    model.has_image = true;
    let render = |model: &Model, w: f32, editor: bool| {
        let mut p = Pixmap::new((w * s) as u32, (380. * s) as u32).unwrap();
        snipping_tool_ul::ui::glass_fill(&mut p);
        let mut icons = IconCache::new();
        let mut c = Canvas {
            pixmap: &mut p,
            scale: s,
            icons: &mut icons,
        };
        model.render_bar(&mut c, w, editor);
        model.render_menu(&mut c, w);
        model.render_upload_feedback(&mut c, w, editor);
        p
    };
    for editor in [false, true] {
        let bw = if editor { 900. } else { w };
        let p = render(&model, bw, editor);
        let tag = if editor { "editor-bar" } else { "home-bar" };
        let _ = writeln!(out, "\n## {tag}, {mode}, {}%", (s * 100.) as u32);
        for t in model.bar_targets(bw, editor) {
            let icon = t.action != Action::ToggleOptions || !editor;
            control(out, &p, s, &format!("{:?}", t.action), t.rect, icon);
        }
        save(&p, dir, tag, mode, s);
    }
    for menu in [
        Menu::Options,
        Menu::Modes,
        Menu::Delay,
        Menu::Hosts,
        Menu::UploadConfirm,
    ] {
        model.menu = menu;
        let p = render(&model, w, false);
        let _ = writeln!(out, "\n## menu {menu:?}, {mode}, {}%", (s * 100.) as u32);
        for t in model.menu_targets(w) {
            let mut r = t.rect;
            if menu == Menu::Options {
                // Measure the leading icon and label, not the trailing control.
                r.w = 200.;
            }
            control(
                out,
                &p,
                s,
                &format!("{:?}", t.action),
                r,
                menu != Menu::UploadConfirm,
            );
        }
        save(&p, dir, &format!("menu-{menu:?}"), mode, s);
    }
    model.menu = Menu::None;
    model.upload_feedback = snipping_tool_ul::upload_feedback::UploadFeedback::Copied {
        url: "https://files.catbox.moe/sample.png".into(),
        reused: false,
    };
    let p = render(&model, w, false);
    let r = model.feedback_rect(w, false);
    let _ = writeln!(out, "\n## upload feedback, {mode}, {}%", (s * 100.) as u32);
    // Both rows share the close target's middle line.
    control(
        out,
        &p,
        s,
        "heading row",
        Rect::new(r.x + 8., r.y + 11., 230., 24.),
        false,
    );
    control(
        out,
        &p,
        s,
        "close target",
        model.feedback_targets(w, false)[0].rect,
        false,
    );
    save(&p, dir, "upload-feedback", mode, s);
}

fn editor(out: &mut String, dir: &Path, mode: &str, s: f32) {
    let cfg = Config::default();
    let (iw, ih) = (640, 360);
    let screen = snipping_tool_ul::capture::CapturedScreen {
        pixels: [70, 80, 96, 255].repeat(iw * ih),
        width: iw as u32,
        height: ih as u32,
        x_offset: 0,
        y_offset: 0,
        scale_factor: 1.,
    };
    let mut state = snipping_tool_ul::state::OverlayState::new(screen, &cfg);
    state.preview_mode = true;
    state.active_tool = snipping_tool_ul::tools::ToolKind::Rectangle;
    let ui = EditorUi::new(&cfg);
    let (w, h) = (900., 600.);
    let mut p = Pixmap::new((w * s) as u32, (h * s) as u32).unwrap();
    ui.draw_base(&mut p, s);
    ui.draw_controls(&mut p, s, &mut state, 1.);
    let (tx, ty) = ((w - 612.) / 2., h - 90.);
    let _ = writeln!(out, "\n## editor dock, {mode}, {}%", (s * 100.) as u32);
    let mut cells = vec![
        ("Undo", tx + 8., 34.),
        ("Redo", tx + 45., 34.),
        ("Color", tx + 442., 48.),
        ("Tools", tx + 498., 48.),
        ("Zoom", tx + 552., 52.),
    ];
    for n in 0..7 {
        cells.push(("tool", tx + 100. + n as f32 * 45., 40.));
    }
    for (name, x, cw) in cells {
        control(
            out,
            &p,
            s,
            &format!("{name} icon"),
            Rect::new(x, ty + 7., cw, 32.),
            false,
        );
        control(
            out,
            &p,
            s,
            &format!("{name} label"),
            Rect::new(x, ty + 36., cw, 16.),
            false,
        );
    }
    save(&p, dir, "editor", mode, s);
}

fn settings(out: &mut String, dir: &Path, mode: &str, s: f32) {
    let mut config = Config::default();
    config.general.theme = mode.into();
    config.general.save_directory = "Pictures / Snipping Tool UL".into();
    let mut ui = SettingsUi::new(config);
    let (w, h) = (settings_ui::WIN_W as f32, settings_ui::WIN_H as f32);
    for tab in 0..3 {
        ui.active_tab = tab;
        let mut p = Pixmap::new((w * s) as u32, (h * s) as u32).unwrap();
        ui.draw(&mut p);
        if tab == 0 {
            let _ = writeln!(out, "\n## settings, {mode}, {}%", (s * 100.) as u32);
            control(out, &p, s, "header", Rect::new(20., 14., 200., 30.), true);
            let cw = (w - 48.) / 4.;
            for i in 0..4 {
                let r = Rect::new(24. + i as f32 * cw + 3., 53., cw - 6., 26.);
                control(out, &p, s, &format!("tab {i}"), r, false);
            }
            let save = Rect::new(w - 24. - 132., h - 54., 132., 34.);
            control(out, &p, s, "Save & Close", save, false);
        }
        save(&p, dir, &format!("settings-tab{tab}"), mode, s);
    }
}

fn about_window(out: &mut String, dir: &Path, mode: &str, s: f32) {
    let (w, h) = (about::WIDTH as f32, about::HEIGHT as f32);
    let mut p = Pixmap::new((w * s) as u32, (h * s) as u32).unwrap();
    about::draw(&mut p, false);
    let _ = writeln!(out, "\n## about, {mode}, {}%", (s * 100.) as u32);
    control(
        out,
        &p,
        s,
        "title row",
        Rect::new(4., 92., w - 8., 36.),
        false,
    );
    control(out, &p, s, "Close", about::close_rect(), false);
    save(&p, dir, "about", mode, s);
}

fn icons(out: &mut String) {
    let _ = writeln!(out, "## icon ink centre offset in 24-unit viewBox");
    let mut cache = IconCache::new();
    for name in [
        "crop",
        "plus",
        "down",
        "right",
        "power",
        "clock",
        "gear",
        "more",
        "check",
        "close",
        "window",
        "screen",
        "preview",
        "history",
        "undo",
        "redo",
        "select",
        "arrow",
        "rectangle",
        "circle",
        "line",
        "pencil",
        "highlight",
        "text",
        "pixelate",
        "step-marker",
        "eyedropper",
        "pin",
        "copy",
        "upload",
        "save",
        "rounded-rect",
        "ocr",
        "spotlight",
        "measurement",
    ] {
        let Some(p) = cache.get_or_render(name, 96, "#000000") else {
            continue;
        };
        let (mut x0, mut y0, mut x1, mut y1) = (96, 96, 0, 0);
        for y in 0..96 {
            for x in 0..96 {
                if p.data()[((y * 96 + x) * 4 + 3) as usize] > 64 {
                    (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x + 1), y1.max(y + 1));
                }
            }
        }
        let dx = ((x0 + x1) as f32 / 2. - 48.) / 4.;
        let dy = ((y0 + y1) as f32 / 2. - 48.) / 4.;
        let flag = if dx.abs() > 0.75 || dy.abs() > 0.75 {
            "  <"
        } else {
            ""
        };
        let _ = writeln!(out, "{name:<14} dx {dx:+5.2} dy {dy:+5.2}{flag}");
    }
}

fn main() {
    let dir = PathBuf::from(
        std::env::args()
            .nth(1)
            .unwrap_or_else(|| "target/alignment-probe".into()),
    );
    std::fs::create_dir_all(&dir).unwrap();
    let mut out = String::new();
    icons(&mut out);
    for (mode, name) in [(ThemeMode::Light, "light"), (ThemeMode::Dark, "dark")] {
        theme::set_mode(mode);
        for s in [1., 1.5] {
            launcher(&mut out, &dir, name, s);
            editor(&mut out, &dir, name, s);
            settings(&mut out, &dir, name, s);
            about_window(&mut out, &dir, name, s);
        }
    }
    std::fs::write(dir.join("alignment.txt"), &out).unwrap();
    print!("{out}");
}
