//! Regressions for the 0.3.14 bug pass. Every test runs headlessly.
use snipping_tool_ul::config::Config;
use snipping_tool_ul::export::{flatten_annotations, save_image_to_path};
use snipping_tool_ul::geometry::{Color, Point, Size};
use snipping_tool_ul::hotkey::parse_binding;
use snipping_tool_ul::overlay::selection::{HitZone, Selection};
use snipping_tool_ul::overlay::toolbar::Toolbar;
use snipping_tool_ul::tools::{
    annotation_bounding_box, hit_test_annotation, resize_annotation, Annotation, ResizeHandle,
};

fn opaque(width: u32, height: u32, rgb: [u8; 3]) -> Vec<u8> {
    (0..width * height)
        .flat_map(|_| [rgb[0], rgb[1], rgb[2], 255])
        .collect()
}

#[test]
fn spotlight_dims_the_export_outside_the_rectangle_only() {
    let (w, h) = (8u32, 8u32);
    let pixels = opaque(w, h, [200, 100, 50]);
    let out = flatten_annotations(
        &pixels,
        w,
        h,
        &[Annotation::Spotlight {
            top_left: Point::new(2.0, 2.0),
            size: Size::new(4.0, 4.0),
        }],
    );
    let px = |x: u32, y: u32| {
        let i = ((y * w + x) * 4) as usize;
        [out[i], out[i + 1], out[i + 2], out[i + 3]]
    };
    // Inside the spotlight the source is restored exactly.
    assert_eq!(px(3, 3), [200, 100, 50, 255]);
    assert_eq!(px(5, 5), [200, 100, 50, 255]);
    // Outside it is darkened by roughly half and stays opaque.
    let outside = px(0, 0);
    assert_eq!(outside[3], 255);
    assert!(outside[0] < 120 && outside[0] > 80, "{outside:?}");
    assert!(outside[1] < 60 && outside[1] > 40, "{outside:?}");
    assert_eq!(px(7, 7), outside);
}

#[test]
fn selection_clamp_drops_the_part_dragged_off_screen() {
    let sel = Selection {
        x: -50.0,
        y: 10.0,
        width: 200.0,
        height: 50.0,
    };
    let r = sel.clamped(1920, 1080);
    assert_eq!((r.x, r.y, r.w, r.h), (0, 10, 150, 50));

    let sel = Selection {
        x: 1800.0,
        y: 1000.0,
        width: 300.0,
        height: 300.0,
    };
    let r = sel.clamped(1920, 1080);
    assert_eq!((r.x, r.y, r.w, r.h), (1800, 1000, 120, 80));

    let sel = Selection {
        x: 10.4,
        y: 10.6,
        width: 20.2,
        height: 20.2,
    };
    let r = sel.clamped(100, 100);
    assert_eq!((r.x, r.y, r.w, r.h), (10, 11, 21, 20));
}

#[test]
fn selection_resize_keeps_tracking_after_the_handle_crosses_the_opposite_edge() {
    let mut sel = Selection {
        x: 50.0,
        y: 50.0,
        width: 100.0,
        height: 100.0,
    };
    // Right edge dragged from x=150 to x=-10, past the left edge: it flips.
    let zone = sel.resize(HitZone::Right, -160.0, 0.0);
    assert_eq!(zone, HitZone::Left);
    assert_eq!((sel.x, sel.width), (-10.0, 60.0));
    // Continuing left with the mirrored zone keeps following the pointer.
    let zone = sel.resize(zone, -10.0, 0.0);
    assert_eq!(zone, HitZone::Left);
    assert_eq!((sel.x, sel.width), (-20.0, 70.0));
    let zone = sel.resize(zone, -10.0, 0.0);
    assert_eq!((sel.x, sel.width), (-30.0, 80.0));
    assert_eq!(zone, HitZone::Left);

    let mut sel = Selection {
        x: 0.0,
        y: 0.0,
        width: 40.0,
        height: 40.0,
    };
    let zone = sel.resize(HitZone::BottomRight, -50.0, -50.0);
    assert_eq!(zone, HitZone::TopLeft);
    assert_eq!(
        (sel.x, sel.y, sel.width, sel.height),
        (-10.0, -10.0, 10.0, 10.0)
    );
}

#[test]
fn text_resize_to_its_own_corner_does_not_move_the_text() {
    let mut ann = Annotation::Text {
        position: Point::new(100.0, 100.0),
        text: "Hello".into(),
        color: Color::red(),
        font_size: 20.0,
    };
    let before = annotation_bounding_box(&ann).unwrap();
    // Drag the bottom-right handle to exactly where it already is, repeatedly.
    for _ in 0..5 {
        let (bx, by, bw, bh) = annotation_bounding_box(&ann).unwrap();
        resize_annotation(
            &mut ann,
            ResizeHandle::BottomRight,
            Point::new(bx + bw, by + bh),
        );
    }
    let after = annotation_bounding_box(&ann).unwrap();
    assert!(
        (before.0 - after.0).abs() < 0.5 && (before.1 - after.1).abs() < 0.5,
        "text crept from {before:?} to {after:?}"
    );
    // Growing the box keeps the top-left corner anchored.
    let (bx, by, bw, bh) = annotation_bounding_box(&ann).unwrap();
    resize_annotation(
        &mut ann,
        ResizeHandle::BottomRight,
        Point::new(bx + bw * 2.0, by + bh * 2.0),
    );
    let grown = annotation_bounding_box(&ann).unwrap();
    assert!(
        (grown.0 - bx).abs() < 1.0 && (grown.1 - by).abs() < 1.0,
        "{grown:?}"
    );
    assert!(grown.3 > bh * 1.5, "{grown:?}");
}

#[test]
fn line_resize_follows_the_pointer_instead_of_lagging_by_the_stroke_padding() {
    let mut ann = Annotation::Line {
        start: Point::new(10.0, 10.0),
        end: Point::new(110.0, 60.0),
        color: Color::red(),
        thickness: 8.0,
    };
    resize_annotation(
        &mut ann,
        ResizeHandle::BottomRight,
        Point::new(210.0, 110.0),
    );
    let Annotation::Line { start, end, .. } = ann else {
        unreachable!()
    };
    assert_eq!(start, Point::new(10.0, 10.0));
    assert_eq!(end, Point::new(210.0, 110.0));

    // A perfectly horizontal line can still be resized along its length.
    let mut flat = Annotation::Line {
        start: Point::new(10.0, 50.0),
        end: Point::new(110.0, 50.0),
        color: Color::red(),
        thickness: 2.0,
    };
    resize_annotation(
        &mut flat,
        ResizeHandle::BottomRight,
        Point::new(160.0, 50.0),
    );
    let Annotation::Line { end, .. } = flat else {
        unreachable!()
    };
    assert_eq!(end, Point::new(160.0, 50.0));
}

#[test]
fn toolbar_stays_on_screen_for_a_selection_near_the_right_edge() {
    let sel = Selection {
        x: 1500.0,
        y: 100.0,
        width: 300.0,
        height: 100.0,
    };
    let toolbar = Toolbar::position_for(&sel, 1920.0, 1080.0);
    assert!(toolbar.x + toolbar.width <= 1920.0, "{}", toolbar.x);
    assert!(toolbar.x >= 0.0);
    let sel = Selection {
        x: 0.0,
        y: 100.0,
        width: 50.0,
        height: 50.0,
    };
    assert_eq!(Toolbar::position_for(&sel, 1920.0, 1080.0).x, 0.0);
}

#[test]
fn hotkey_parser_rejects_bare_letters_and_double_keys() {
    assert!(parse_binding("Ctrl+A+B").is_err());
    assert!(parse_binding("S").is_err());
    assert!(parse_binding("Shift+S").is_ok());
    assert!(parse_binding("PrintScreen").is_ok());
    assert!(parse_binding("F1").is_ok());
    assert!(parse_binding("Ctrl+Shift+S").is_ok());
}

#[test]
fn nan_thickness_falls_back_instead_of_propagating() {
    let mut cfg = Config::default();
    cfg.general.default_thickness = f32::NAN;
    assert_eq!(cfg.clamped_thickness(), 3.0);
    cfg.general.default_thickness = 50.0;
    assert_eq!(cfg.clamped_thickness(), 20.0);
}

#[test]
fn single_point_pencil_and_flat_ellipse_hit_tests() {
    let dot = Annotation::Pencil {
        points: vec![Point::new(40.0, 40.0)],
        color: Color::red(),
        thickness: 4.0,
    };
    assert!(hit_test_annotation(&dot, &Point::new(42.0, 41.0), 6.0));
    assert!(!hit_test_annotation(&dot, &Point::new(60.0, 40.0), 6.0));

    // 300 x 30 ellipse: the interior far from the outline must not hit.
    let ellipse = Annotation::Ellipse {
        center: Point::new(200.0, 100.0),
        radius_x: 150.0,
        radius_y: 15.0,
        color: Color::red(),
        thickness: 2.0,
    };
    assert!(!hit_test_annotation(
        &ellipse,
        &Point::new(200.0, 100.0),
        8.0
    ));
    assert!(!hit_test_annotation(
        &ellipse,
        &Point::new(130.0, 100.0),
        8.0
    ));
    assert!(hit_test_annotation(
        &ellipse,
        &Point::new(350.0, 100.0),
        8.0
    ));
    assert!(hit_test_annotation(&ellipse, &Point::new(200.0, 86.0), 8.0));
}

#[test]
fn save_to_path_never_leaves_a_partial_file_behind() {
    let dir = std::env::temp_dir().join(format!("ul-save-tests-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let pixels = opaque(6, 4, [10, 20, 30]);

    for name in ["a.png", "b.jpg", "c.webp"] {
        let path = dir.join(name);
        std::fs::write(&path, b"old contents").unwrap();
        save_image_to_path(pixels.clone(), 6, 4, &path).unwrap();
        let img = image::open(&path).unwrap();
        assert_eq!(img.width(), 6);
        assert!(!dir.join(format!("{name}.part")).exists());
    }

    // Failure path: the destination folder does not exist.
    let missing = dir.join("missing").join("d.png");
    assert!(save_image_to_path(pixels.clone(), 6, 4, &missing).is_err());
    assert!(!dir.join("missing").exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn history_dedupe_skips_an_identical_second_write() {
    let dir = std::env::temp_dir().join(format!(
        "ul-history-dedupe-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    std::env::set_var("SNIPPING_TOOL_UL_CONFIG_DIR", &dir);
    assert_eq!(
        snipping_tool_ul::history::history_dir(),
        Some(dir.join("history"))
    );
    let pixels = opaque(5, 5, [1, 2, 3]);
    let first = snipping_tool_ul::history::save_to_history_if_new(&pixels, 5, 5).unwrap();
    assert!(first.is_some());
    let second = snipping_tool_ul::history::save_to_history_if_new(&pixels, 5, 5).unwrap();
    assert!(second.is_none());
    let other = opaque(5, 5, [3, 2, 1]);
    let third = snipping_tool_ul::history::save_to_history_if_new(&other, 5, 5).unwrap();
    assert!(third.is_some());
    assert_eq!(snipping_tool_ul::history::list_history().len(), 2);
    let _ = std::fs::remove_dir_all(&dir);
}
