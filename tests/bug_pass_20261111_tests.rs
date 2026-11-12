use snipping_tool_ul::{
    capture::{CapturedScreen, ScreenRect},
    config::Config,
    export,
    geometry::{outward_span, Point, Size},
    overlay::selection::Selection,
    state::OverlayState,
    tools::Annotation,
};
use std::path::PathBuf;

fn rect(x: i32, y: i32, width: u32, height: u32) -> ScreenRect {
    ScreenRect {
        x,
        y,
        width,
        height,
    }
}

#[test]
fn repeat_area_must_lie_on_monitors_not_just_inside_the_desktop_box() {
    // A 4K monitor with a shorter 1080p one to its right, and one to the left
    // at a negative origin: the bounding box has an empty band below the 1080p.
    let monitors = [
        rect(0, 0, 3840, 2160),
        rect(3840, 0, 1920, 1080),
        rect(-1920, 0, 1920, 1080),
    ];
    assert!(rect(3800, 100, 100, 200).covered_by(&monitors));
    assert!(rect(-50, 10, 100, 100).covered_by(&monitors));
    assert!(rect(0, 0, 3840, 2160).covered_by(&monitors));
    // Crosses into the band under the 1080p monitor.
    assert!(!rect(3800, 1000, 100, 200).covered_by(&monitors));
    // Wholly in the band, and wholly off every monitor.
    assert!(!rect(4000, 1500, 10, 10).covered_by(&monitors));
    assert!(!rect(9000, 0, 10, 10).covered_by(&monitors));
    // A monitor that was unplugged.
    assert!(!rect(-50, 10, 100, 100).covered_by(&monitors[..2]));
    assert!(!rect(10, 10, 0, 5).covered_by(&monitors));
    assert!(!rect(10, 10, 5, 5).covered_by(&[]));
}

#[test]
fn outward_span_covers_every_partly_touched_pixel() {
    assert_eq!(outward_span(10.0, 20.0), (10, 20));
    assert_eq!(outward_span(10.9, 100.9), (10, 102));
    assert_eq!(outward_span(-2.4, 8.7), (0, 7));
    assert_eq!(outward_span(-20.0, 5.0), (0, 0));
    assert_eq!(outward_span(5.0, -3.0), (5, 0));
    assert_eq!(outward_span(0.999999, 6.5), (0, 8));
    // Float casts saturate; every caller then clips the span to the image.
    assert_eq!(outward_span(1e30, 1e30), (u32::MAX, u32::MAX));
    assert!(snipping_tool_ul::canvas::source_region(
        Point::new(1e30, 0.),
        Size::new(1e30, 5.),
        100,
        80
    )
    .is_none());
}

fn state() -> OverlayState {
    let mut pixels = Vec::with_capacity(100 * 80 * 4);
    for i in 0..100 * 80u32 {
        pixels.extend_from_slice(&[
            (i * 7 % 256) as u8,
            (i * 13 % 256) as u8,
            (i % 97) as u8,
            255,
        ]);
    }
    let mut o = OverlayState::new(
        CapturedScreen {
            pixels,
            width: 100,
            height: 80,
            x_offset: 0,
            y_offset: 0,
            scale_factor: 1.,
        },
        &Config::default(),
    );
    o.preview_mode = true;
    o.selection = Some(Selection {
        x: 0.,
        y: 0.,
        width: 100.,
        height: 80.,
    });
    o
}

fn exported_with(effect: Annotation) -> Vec<u8> {
    let mut o = state();
    o.annotations.push(effect);
    o.flattened_selection().unwrap().0
}

#[test]
fn fractional_pixelate_redacts_its_partly_covered_right_and_bottom_edge() {
    let pixelate = |x, y, w, h| Annotation::Pixelate {
        top_left: Point::new(x, y),
        size: Size::new(w, h),
        block_size: 4,
    };
    // 10.9 + 20.2 = 31.1, so columns and rows 10 through 31 are touched.
    let fractional = exported_with(pixelate(10.9, 10.9, 20.2, 20.2));
    let whole = exported_with(pixelate(10.0, 10.0, 22.0, 22.0));
    assert_eq!(fractional, whole);
    // The old truncation stopped at 30: column 30 and 31 then kept the original.
    let original = state().flattened_selection().unwrap().0;
    let at = |x: usize, y: usize| (y * 100 + x) * 4;
    let p = at(31, 15);
    assert_ne!(&fractional[p..p + 4], &original[p..p + 4]);
    let below = at(15, 31);
    assert_ne!(&fractional[below..below + 4], &original[below..below + 4]);
    let outside = at(32, 15);
    assert_eq!(
        &fractional[outside..outside + 4],
        &original[outside..outside + 4]
    );
}

#[test]
fn fractional_spotlight_reveals_the_same_pixels_as_its_whole_pixel_cover() {
    let spotlight = |x, y, w, h| Annotation::Spotlight {
        top_left: Point::new(x, y),
        size: Size::new(w, h),
    };
    assert_eq!(
        exported_with(spotlight(20.5, 10.25, 30.0, 30.0)),
        exported_with(spotlight(20.0, 10.0, 31.0, 31.0))
    );
}

#[test]
fn save_names_without_an_image_extension_get_png() {
    let named = |s: &str| export::with_image_extension(PathBuf::from(s));
    assert_eq!(named(r"C:\shots\a.png"), PathBuf::from(r"C:\shots\a.png"));
    assert_eq!(named(r"C:\shots\a.JPG"), PathBuf::from(r"C:\shots\a.JPG"));
    assert_eq!(named("a.jpeg"), PathBuf::from("a.jpeg"));
    assert_eq!(named("a.webp"), PathBuf::from("a.webp"));
    assert_eq!(named("a.bmp"), PathBuf::from("a.bmp.png"));
    assert_eq!(named("notes.final"), PathBuf::from("notes.final.png"));
    assert_eq!(named("shot"), PathBuf::from("shot.png"));
}

#[test]
fn settings_show_and_save_the_thickness_the_editor_actually_draws() {
    for (raw, shown) in [(100.0, 20.0), (0.0, 1.0), (f32::NAN, 3.0), (4.5, 4.5)] {
        let mut config = Config::default();
        config.general.default_thickness = raw;
        let ui = snipping_tool_ul::settings_ui::SettingsUi::new(config);
        assert_eq!(ui.config.general.default_thickness, shown, "raw={raw}");
    }
}

include!("support/pin_frame_before_20261111.rs");

#[test]
fn row_wise_pin_frame_matches_the_three_pass_build_it_replaced() {
    let accent = (12, 99, 201);
    for (w, h, short) in [
        (0u32, 0u32, 0usize),
        (1, 1, 0),
        (37, 5, 0),
        (5, 37, 0),
        (64, 48, 3 * 64 * 4 + 6),
        (300, 200, 0),
    ] {
        let mut pixels: Vec<u8> = (0..w as usize * h as usize * 4)
            .map(|i| (i * 37 % 253) as u8)
            .collect();
        if short > 0 {
            pixels.truncate(short);
        }
        assert_eq!(
            snipping_tool_ul::pin::framed_pixels(&pixels, w, h, accent),
            pin_frame_before(&pixels, w, h, accent),
            "{w}x{h}, source bytes {}",
            pixels.len()
        );
    }
}
