use snipping_tool_ul::capture::ScreenRect;
use snipping_tool_ul::preview::{image_point, present};

#[test]
fn negative_monitor_selection_and_reversed_drag_are_physical() {
    let bounds = ScreenRect {
        x: -3840,
        y: 0,
        width: 11520,
        height: 2160,
    };
    assert_eq!(
        ScreenRect::between((-3200, 700), (-3700, 200)).intersect(bounds),
        Some(ScreenRect {
            x: -3700,
            y: 200,
            width: 500,
            height: 500
        })
    );
    assert_eq!(
        ScreenRect::between((-4000, -20), (-3800, 100)).intersect(bounds),
        Some(ScreenRect {
            x: -3840,
            y: 0,
            width: 40,
            height: 100
        })
    );
    assert_eq!(
        ScreenRect::between((9000, 0), (10000, 100)).intersect(bounds),
        None
    );
    assert_eq!(ScreenRect::between((0, 0), (0, 5)).intersect(bounds), None);
}

#[test]
fn extreme_coordinates_do_not_wrap_when_clipped() {
    let bounds = ScreenRect {
        x: -3840,
        y: 0,
        width: 11520,
        height: 2160,
    };
    let extreme = ScreenRect::between((i32::MIN, i32::MIN), (i32::MAX, i32::MAX));
    assert_eq!(extreme.intersect(bounds), Some(bounds));
    assert!("0,0,0,20".parse::<ScreenRect>().is_err());
    assert_eq!("-100,200,640,400".parse::<ScreenRect>().unwrap().x, -100);
}

#[test]
fn resized_preview_letterbox_keeps_pixel_and_pointer_coordinates_aligned() {
    let mut pixmap = tiny_skia::Pixmap::new(2, 1).unwrap();
    pixmap
        .data_mut()
        .copy_from_slice(&[255, 0, 0, 255, 0, 255, 0, 255]);
    let mut output = vec![0; 16];
    present(&pixmap, &mut output, 4, 4);
    assert_eq!(&output[0..4], &[0x18181c; 4]);
    assert_eq!(&output[4..8], &[0xff0000, 0xff0000, 0x00ff00, 0x00ff00]);
    assert_eq!(&output[12..16], &[0x18181c; 4]);
    let point = image_point(3.0, 2.0, 2, 1, 4, 4);
    assert_eq!((point.x, point.y), (1.5, 0.5));
}

#[test]
fn tiny_preview_fits_every_toolbar_target_below_image() {
    use snipping_tool_ul::{
        capture::CapturedScreen, config::Config, geometry::Point, overlay::selection::Selection,
        state::OverlayState,
    };
    let config = Config::default();
    let mut state = OverlayState::new(
        CapturedScreen {
            pixels: vec![255; 200 * 100 * 4],
            width: 200,
            height: 100,
            x_offset: 0,
            y_offset: 0,
            scale_factor: 1.0,
        },
        &config,
    );
    let count = state.visible_buttons.len();
    let (width, height) = snipping_tool_ul::preview::canvas_size(200, 100, count);
    state.canvas_height = height;
    assert!(state
        .selection_hit(Point::new(500.0, 250.0), true)
        .is_none());
    assert_eq!(
        state.selection_hit(Point::new(1.0, 1.0), true),
        Some(snipping_tool_ul::overlay::selection::HitZone::Inside)
    );
    let selection = Selection {
        x: 0.0,
        y: 0.0,
        width: 200.0,
        height: 100.0,
    };
    let toolbar = state.toolbar(&selection);
    assert!(toolbar.y >= 100.0);
    assert!(toolbar.x + toolbar.width <= width as f32);
    assert!(toolbar.y + toolbar.height <= height as f32);
    for i in 0..count {
        let target = Point::new(toolbar.x + 24.0 + i as f32 * 40.0, toolbar.y + 24.0);
        assert_eq!(toolbar.hit_test_dynamic(target, count), Some(i));
    }
}
