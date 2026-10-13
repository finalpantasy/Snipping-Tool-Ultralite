//! Lazy glyph loading must render exactly what a full fontdue parse renders.
use snipping_tool_ul::font::LazyFont;

const SETTINGS: fontdue::FontSettings = fontdue::FontSettings {
    collection_index: 0,
    scale: 40.0,
    load_substitutions: false,
};

fn sources() -> Vec<(&'static str, Vec<u8>)> {
    let fonts = vec![("embedded", include_bytes!("../assets/font.ttf").to_vec())];
    fonts
}

fn sample() -> Vec<char> {
    // Out of order on purpose so blocks load in a mixed sequence.
    let mut chars: Vec<char> = (0x2000u32..0x2200)
        .chain(0x20..0x600)
        .chain(0xFFF0..0x10000)
        .chain([0x1F600, 0x10FFFD, 0xE000])
        .filter_map(char::from_u32)
        .collect();
    chars.push('\u{0}');
    chars
}

#[test]
fn lazy_fonts_match_full_parse_metrics_and_coverage() {
    for (name, bytes) in sources() {
        let full = fontdue::Font::from_bytes(bytes.as_slice(), SETTINGS).unwrap();
        let lazy = LazyFont::new(bytes.clone()).expect(name);
        for px in [9.0f32, 11.0, 13.0, 19.5, 40.0] {
            assert_eq!(
                full.horizontal_line_metrics(px),
                lazy.horizontal_line_metrics(px),
                "{name} line metrics at {px}"
            );
            for &ch in &sample() {
                assert_eq!(
                    full.metrics(ch, px),
                    lazy.metrics(ch, px),
                    "{name} {ch:?} {px}"
                );
                assert_eq!(
                    full.rasterize(ch, px),
                    lazy.rasterize(ch, px),
                    "{name} {ch:?} {px}"
                );
            }
        }
        // Whole strings resolve through one parsed font.
        let text = "Settings Ctrl+Shift+S \u{2026} \u{00d7} \u{0416}\u{03a9}";
        let font = lazy.for_text(text);
        for ch in text.chars() {
            assert_eq!(
                full.rasterize(ch, 13.),
                font.rasterize(ch, 13.),
                "{name} {ch:?}"
            );
        }
    }
}
