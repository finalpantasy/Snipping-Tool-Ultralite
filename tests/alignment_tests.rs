//! Headless pixel checks for control alignment. No windows are created.
use snipping_tool_ul::{
    icons::IconCache,
    ui::{Canvas, Rect},
};
use tiny_skia::Pixmap;

/// Most common bottom ink row across columns: the baseline, ignoring descenders.
fn baseline(p: &Pixmap) -> i32 {
    let (w, h) = (p.width() as i32, p.height() as i32);
    let mut counts = std::collections::HashMap::new();
    for x in 0..w {
        let bottom = (0..h)
            .rev()
            .find(|&y| p.data()[((y * w + x) * 4 + 3) as usize] > 128);
        if let Some(y) = bottom {
            *counts.entry(y).or_insert(0) += 1;
        }
    }
    counts.into_iter().max_by_key(|&(y, n)| (n, -y)).unwrap().0
}

fn label_baseline(label: &str, scale: f32) -> i32 {
    let mut p = Pixmap::new((120. * scale) as u32, (32. * scale) as u32).unwrap();
    let mut icons = IconCache::new();
    let mut c = Canvas {
        pixmap: &mut p,
        scale,
        icons: &mut icons,
    };
    c.centered_in(Rect::new(0., 0., 120., 32.), label, 13., (0, 0, 0));
    baseline(&p)
}

#[test]
fn labels_with_and_without_descenders_share_a_baseline() {
    for scale in [1., 1.25, 1.5, 2.] {
        let expected = label_baseline("New", scale);
        for label in ["Delay", "Copy", "Rectangle", "Upload", "Options", "Save"] {
            assert_eq!(
                label_baseline(label, scale),
                expected,
                "{label} at {scale}x sits on a different baseline than New"
            );
        }
    }
}

#[test]
fn icons_used_in_buttons_are_centred_in_their_box() {
    let mut cache = IconCache::new();
    for name in [
        "undo", "redo", "check", "close", "plus", "gear", "upload", "save",
    ] {
        let p = cache.get_or_render(name, 96, "#000000").unwrap();
        let (mut y0, mut y1, mut x0, mut x1) = (96, 0, 96, 0);
        for y in 0..96 {
            for x in 0..96 {
                if p.data()[((y * 96 + x) * 4 + 3) as usize] > 64 {
                    (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x + 1), y1.max(y + 1));
                }
            }
        }
        // Within 3 of 96 pixels: under one pixel at the 16-24 pixel sizes used.
        assert!(
            (x0 + x1 - 96i32).abs() <= 6,
            "{name} is off-centre horizontally"
        );
        assert!(
            (y0 + y1 - 96i32).abs() <= 6,
            "{name} is off-centre vertically"
        );
    }
}
