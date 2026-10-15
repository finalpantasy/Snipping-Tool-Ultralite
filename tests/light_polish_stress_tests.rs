//! Edge cases for the light-polish features: random OCR text, every hotkey
//! shape, extreme zoom and tiny windows. Headless; no windows or clipboard.
use snipping_tool_ul::{
    capture::CapturedScreen,
    config::Config,
    editor_ui::EditorUi,
    geometry::Point,
    hotkey::{parse_binding, repeat_binding},
    ocr::Word,
    preview::EditorView,
    preview_render::PreviewRenderer,
    redact::private_areas,
    state::OverlayState,
};

/// Small deterministic generator so failures reproduce.
struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 33
    }
    fn pick<'a>(&mut self, items: &'a [&'a str]) -> &'a str {
        items[self.next() as usize % items.len()]
    }
}

#[test]
fn random_ocr_text_never_panics_and_areas_stay_sane() {
    let pieces = [
        "a", "Zz", "@", ".", "-", "(", ")", "+", "/", "0", "12", "345", "6789", "é", "日本", "_",
        "sk-", "ghp_", "com", "x", "..", "@@", "1-2", "2026", " ",
    ];
    let mut rng = Lcg(7);
    for round in 0..2000 {
        let words: Vec<Word> = (0..(rng.next() % 12))
            .map(|i| {
                let text: String = (0..(rng.next() % 9)).map(|_| rng.pick(&pieces)).collect();
                Word {
                    text,
                    line: (i / 4) as usize,
                    x: (rng.next() % 900) as f32 - 50.,
                    y: (rng.next() % 600) as f32,
                    w: (rng.next() % 200) as f32,
                    h: (rng.next() % 40) as f32,
                }
            })
            .collect();
        for area in private_areas(&words) {
            assert!(area.w >= 0. && area.h >= 0., "round {round}: {area:?}");
            let padded = area.padded(3., 800., 600.);
            assert!(padded.x >= 0. && padded.y >= 0., "round {round}");
            assert!(padded.x + padded.w <= 800. + 0.001, "round {round}");
        }
    }
}

#[test]
fn every_capture_shortcut_gets_a_distinct_valid_repeat_or_none() {
    let mut keys: Vec<String> = ('A'..='Z').map(String::from).collect();
    keys.extend(('0'..='9').map(String::from));
    keys.extend((1..=12).map(|n| format!("F{n}")));
    keys.push("PrintScreen".into());
    let mods = ["Ctrl", "Shift", "Alt", "Super"];
    for key in &keys {
        for mask in 0..16u8 {
            let mut parts: Vec<&str> = (0..4)
                .filter(|b| mask & (1 << b) != 0)
                .map(|b| mods[b])
                .collect();
            parts.push(key);
            let capture = parts.join("+");
            if parse_binding(&capture).is_err() {
                continue;
            }
            if let Some(repeat) = repeat_binding(&capture) {
                assert!(parse_binding(&repeat).is_ok(), "{capture} -> {repeat}");
                assert_ne!(
                    parse_binding(&repeat).unwrap(),
                    parse_binding(&capture).unwrap(),
                    "{capture}"
                );
                for reserved in ["Alt+PrintScreen", "Shift+Alt+PrintScreen", "Alt+F4"] {
                    assert_ne!(
                        parse_binding(&repeat).unwrap(),
                        parse_binding(reserved).unwrap(),
                        "{capture} -> {repeat} is a Windows shortcut"
                    );
                }
            }
        }
    }
}

fn state() -> OverlayState {
    let mut s = OverlayState::new(
        CapturedScreen {
            pixels: vec![255; 40 * 30 * 4],
            width: 40,
            height: 30,
            x_offset: 0,
            y_offset: 0,
            scale_factor: 1.,
        },
        &Config::default(),
    );
    s.preview_mode = true;
    s.selection = Some(
        snipping_tool_ul::overlay::selection::Selection::from_points(
            Point::new(0., 0.),
            Point::new(40., 30.),
        ),
    );
    s
}

#[test]
fn editor_draws_at_extreme_zoom_and_tiny_windows_without_panicking() {
    let mut s = state();
    let ui = EditorUi::new(&Config::default());
    for (w, h) in [(1, 1), (2, 3), (120, 80), (660, 390), (3000, 2000)] {
        for zoom in [1e-6, 0.01, 1., 64., 1e6] {
            for dpi in [1., 1.5, 3.] {
                let mut frame = tiny_skia::Pixmap::new(w, h).unwrap();
                let mut view = EditorView::default();
                view.zoom = zoom;
                ui.draw_base(&mut frame, dpi as f32);
                PreviewRenderer::default().draw(&s, &mut frame, &view, dpi);
                ui.draw_controls(&mut frame, dpi as f32, &mut s, zoom);
            }
        }
    }
    // Hit testing a minimized (0x0) window and non-finite pointers.
    for p in [
        Point::new(0., 0.),
        Point::new(f32::NAN, 5.),
        Point::new(-1e9, 1e9),
    ] {
        let _ = ui.hit(p, 0., 0., &s);
        let _ = ui.hit(p, 940., 612., &s);
    }
}
