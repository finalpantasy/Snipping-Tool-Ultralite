use snipping_tool_ul::{
    capture::CapturedScreen,
    config::{Config, ShortcutsConfig},
    export,
    font::{self, Fallback},
    state::OverlayState,
};

#[test]
fn jpeg_puts_transparent_pixels_on_white_not_black() {
    let img = image::RgbaImage::from_raw(
        4,
        1,
        vec![
            10, 20, 30, 255, // opaque: unchanged
            0, 0, 0, 0, // fully transparent black: white
            200, 0, 0, 0, // transparent with stored color: still white
            255, 0, 0, 128, // half red: pink
        ],
    )
    .unwrap();
    assert_eq!(
        export::rgb_on_white(&img).as_raw(),
        &[10, 20, 30, 255, 255, 255, 255, 255, 255, 255, 127, 127]
    );

    let dir = std::env::temp_dir().join(format!("ul-jpeg-white-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("padded.jpg");
    export::save_image_to_path([0, 0, 0, 0].repeat(16 * 16), 16, 16, &path).unwrap();
    let decoded = image::open(&path).unwrap().to_rgb8();
    let _ = std::fs::remove_dir_all(&dir);
    assert!(decoded.pixels().all(|p| p.0.iter().all(|&c| c >= 250)));
}

#[test]
fn assigning_a_taken_letter_swaps_the_two_tools() {
    let mut keys = ShortcutsConfig::default();
    // Arrow (a) takes R from Rectangle, which gets A.
    keys.assign(1, "r".into());
    assert_eq!((keys.arrow.as_str(), keys.rectangle.as_str()), ("r", "a"));
    // Case does not hide a clash.
    keys.assign(0, "R".into());
    assert_eq!((keys.select.as_str(), keys.arrow.as_str()), ("R", "v"));
    // A free letter, and a tool's own letter, change nothing else.
    let before = keys.clone();
    keys.assign(5, "q".into());
    keys.assign(5, "q".into());
    let changed: Vec<usize> = (0..14)
        .filter(|&i| keys.entries()[i].2 != before.entries()[i].2)
        .collect();
    assert_eq!(changed, vec![5]);
    let all: Vec<String> = keys.entries().iter().map(|e| e.2.to_lowercase()).collect();
    let mut unique = all.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), all.len(), "{all:?}");
    // Out of range is ignored.
    keys.assign(99, "z".into());
}

fn system_font(name: &str) -> Option<std::path::PathBuf> {
    let windir = std::env::var_os("WINDIR")?;
    let path = std::path::PathBuf::from(windir).join("Fonts").join(name);
    path.exists().then_some(path)
}

fn leaked(paths: Vec<std::path::PathBuf>) -> &'static [Fallback] {
    Box::leak(paths.into_iter().map(Fallback::new).collect::<Box<[_]>>())
}

#[test]
fn ime_text_draws_real_glyphs_from_a_system_font() {
    let Some(collection) = system_font("YuGothM.ttc").or_else(|| system_font("msyh.ttc")) else {
        eprintln!("skipped: no Japanese or Chinese system font");
        return;
    };
    let notdef = font::FONT.metrics('日', 40.);
    let fonts = font::text_fonts_from("Ab 日本語", leaked(vec![collection]));
    let han = fonts.metrics('日', 40.);
    assert!(han.width > 20 && han.height > 20, "{han:?}");
    assert_ne!((han.width, han.height), (notdef.width, notdef.height));
    let (m, mask) = fonts.rasterize('本', 40.);
    assert_eq!(mask.len(), m.width * m.height);
    assert!(mask.iter().filter(|&&a| a > 128).count() > 50);
    // Latin keeps Open Sans, glyph for glyph, and the line keeps its metrics.
    assert_eq!(
        fonts.metrics('A', 40.).advance_width,
        font::FONT.metrics('A', 40.).advance_width
    );
    assert_eq!(
        fonts.horizontal_line_metrics(40.).map(|m| m.ascent),
        font::FONT.horizontal_line_metrics(40.).map(|m| m.ascent)
    );
}

#[test]
fn hangul_comes_from_the_korean_font_after_a_font_without_it() {
    let (Some(korean), Some(symbols)) = (system_font("malgun.ttf"), system_font("seguisym.ttf"))
    else {
        eprintln!("skipped: Malgun Gothic or Segoe UI Symbol missing");
        return;
    };
    let missing = std::path::PathBuf::from("Z:\\no such folder\\none.ttf");
    let fonts = font::text_fonts_from("한글 ★", leaked(vec![missing, symbols, korean]));
    assert!(fonts.metrics('한', 30.).width > 10);
    assert!(fonts.metrics('★', 30.).width > 10);
}

#[test]
fn a_missing_fallback_leaves_open_sans_without_failing() {
    let fonts = font::text_fonts_from(
        "日本",
        leaked(vec![std::path::PathBuf::from(
            "Z:\\no such folder\\none.ttc",
        )]),
    );
    let notdef = font::FONT.metrics('日', 24.);
    let m = fonts.metrics('日', 24.);
    assert_eq!((m.width, m.height), (notdef.width, notdef.height));
}

#[test]
fn composing_text_is_shown_but_never_committed() {
    let mut o = OverlayState::new(
        CapturedScreen {
            pixels: [0, 0, 0, 255].repeat(4),
            width: 2,
            height: 2,
            x_offset: 0,
            y_offset: 0,
            scale_factor: 1.,
        },
        &Config::default(),
    );
    o.text_input_active = true;
    o.text_input_buffer = "Note ".into();
    o.text_preedit = "にほん".into();
    assert_eq!(o.text_input_display(), "Note にほん");
    o.commit_text();
    assert!(o.text_preedit.is_empty());
    match o.annotations.last() {
        Some(snipping_tool_ul::tools::Annotation::Text { text, .. }) => assert_eq!(text, "Note "),
        other => panic!("{other:?}"),
    }
}
