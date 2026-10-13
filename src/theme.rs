//! Graphite interface palette and the process-global active mode.
//! Every rendered surface reads its colors through these accessors so a
//! single `set_mode` call re-themes the whole app.

use std::sync::atomic::{AtomicU8, Ordering};

type Rgb = (u8, u8, u8);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ThemeMode {
    Dark,
    Light,
}

static ACTIVE: AtomicU8 = AtomicU8::new(0); // 0 = Dark, 1 = Light

pub fn set_mode(mode: ThemeMode) {
    let v = match mode {
        ThemeMode::Dark => 0,
        ThemeMode::Light => 1,
    };
    ACTIVE.store(v, Ordering::Relaxed);
}

pub fn mode() -> ThemeMode {
    if ACTIVE.load(Ordering::Relaxed) == 1 {
        ThemeMode::Light
    } else {
        ThemeMode::Dark
    }
}

struct Palette {
    bg_0: Rgb,
    bg_1: Rgb,
    bg_2: Rgb,
    bg_3: Rgb,
    bg_4: Rgb,
    border: Rgb,
    border_strong: Rgb,
    text_normal: Rgb,
    text_soft: Rgb,
    text_muted: Rgb,
    text_faint: Rgb,
    text_on_accent: Rgb,
    accent: Rgb,
    accent_alt: Rgb,
    warm: Rgb,
    red: Rgb,
    orange: Rgb,
    violet: Rgb,
}

// Graphite interface palette — dark (default).
const DARK: Palette = Palette {
    bg_0: (15, 25, 29),
    bg_1: (20, 32, 37),
    bg_2: (29, 45, 51),
    bg_3: (40, 60, 67),
    bg_4: (57, 79, 86),
    border: (74, 99, 107),
    border_strong: (98, 127, 136),
    text_normal: (242, 245, 250),
    text_soft: (230, 235, 244),
    text_muted: (217, 225, 238),
    text_faint: (169, 181, 199),
    text_on_accent: (249, 251, 248),
    accent: (0, 124, 145),
    accent_alt: (83, 200, 214),
    warm: (0xf0, 0xc6, 0x74),
    red: (0xff, 0x6e, 0x7a),
    orange: (0xf7, 0xa3, 0x5c),
    violet: (0xb7, 0x8c, 0xff),
};

// Graphite interface palette — light.
const LIGHT: Palette = Palette {
    bg_0: (245, 249, 248),
    bg_1: (236, 244, 243),
    bg_2: (220, 233, 232),
    bg_3: (201, 220, 219),
    bg_4: (181, 204, 204),
    border: (148, 178, 180),
    border_strong: (120, 156, 161),
    text_normal: (28, 37, 53),
    text_soft: (48, 60, 79),
    text_muted: (68, 81, 101),
    text_faint: (116, 131, 154),
    text_on_accent: (0xf9, 0xfb, 0xf8),
    accent: (0, 124, 145),
    accent_alt: (0, 106, 124),
    warm: (0xa4, 0x66, 0x00),
    red: (0xc8, 0x32, 0x4c),
    orange: (0xb6, 0x58, 0x00),
    violet: (0x73, 0x57, 0xb8),
};

fn active() -> &'static Palette {
    match mode() {
        ThemeMode::Dark => &DARK,
        ThemeMode::Light => &LIGHT,
    }
}

macro_rules! token {
    ($name:ident) => {
        pub fn $name() -> Rgb {
            active().$name
        }
    };
}
token!(bg_0);
token!(bg_1);
token!(bg_2);
token!(bg_3);
token!(bg_4);
token!(border);
token!(border_strong);
token!(text_normal);
token!(text_soft);
token!(text_muted);
token!(text_faint);
token!(text_on_accent);
token!(accent);
token!(accent_alt);
token!(warm);
token!(red);
token!(orange);
token!(violet);

/// Tint over Windows' native acrylic, bounded for label contrast over any backdrop.
pub fn glass_alpha() -> u8 {
    match mode() {
        ThemeMode::Dark => 180,
        ThemeMode::Light => 210,
    }
}

/// Convert a token to a `tiny_skia::Color` at the given alpha (0.0..=1.0).
pub fn skia(rgb: Rgb, alpha: f32) -> tiny_skia::Color {
    tiny_skia::Color::from_rgba(
        rgb.0 as f32 / 255.0,
        rgb.1 as f32 / 255.0,
        rgb.2 as f32 / 255.0,
        alpha,
    )
    .unwrap_or(tiny_skia::Color::WHITE)
}

/// Convert a token to the crate's own `geometry::Color` at the given alpha.
pub fn gcolor(rgb: Rgb, alpha: f32) -> crate::geometry::Color {
    crate::geometry::Color::new(
        rgb.0 as f32 / 255.0,
        rgb.1 as f32 / 255.0,
        rgb.2 as f32 / 255.0,
        alpha,
    )
}

/// Lowercase `#rrggbb` string (for the SVG icon renderer).
pub fn hex(rgb: Rgb) -> String {
    format!("#{:02x}{:02x}{:02x}", rgb.0, rgb.1, rgb.2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dark_and_light_differ_and_match_spec() {
        set_mode(ThemeMode::Dark);
        assert_eq!(accent(), (0, 124, 145));
        assert_eq!(bg_0(), (15, 25, 29));
        assert_eq!(text_normal(), (242, 245, 250));

        set_mode(ThemeMode::Light);
        assert_eq!(accent(), (0, 124, 145));
        assert_eq!(bg_0(), (245, 249, 248));
        assert_eq!(text_normal(), (28, 37, 53));

        set_mode(ThemeMode::Dark);
    }

    #[test]
    fn hex_formats_lowercase_six_digits() {
        assert_eq!(hex((0x63, 0xf2, 0xab)), "#63f2ab");
    }

    #[test]
    fn skia_applies_alpha() {
        let c = skia((0xff, 0x00, 0x00), 0.5);
        assert!((c.red() - 1.0).abs() < 0.01);
        assert!((c.alpha() - 0.5).abs() < 0.01);
    }
}
