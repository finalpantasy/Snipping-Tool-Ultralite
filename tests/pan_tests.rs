//! Production view transforms only: no windows, input injection or desktop pixels.
use snipping_tool_ul::{
    geometry::{Color, Point},
    preview::EditorView,
    tools::{hit_test_annotation, Annotation},
};

#[test]
fn middle_drag_moves_the_view_one_physical_pixel_per_pointer_pixel_at_any_zoom_or_dpi() {
    for dpi in [1., 1.25, 1.5, 2.] {
        let (vw, vh) = ((940. * dpi) as u32, (612. * dpi) as u32);
        for zoom in [0.25, 1., 3., 16.] {
            let mut view = EditorView::default();
            view.zoom = zoom;
            let (x, y) = (450. * dpi, 200. * dpi);
            let before = view.framed_layout(1920, 1080, vw, vh, dpi);
            let point = view.framed_point(x, y, 1920, 1080, vw, vh, dpi);
            assert!(view.begin_pan(x, y, vw, vh, dpi));
            assert!(view.drag_pan(x + 60. * dpi, y - 20. * dpi));
            let after = view.framed_layout(1920, 1080, vw, vh, dpi);
            assert!((after.x - before.x - 60. * dpi).abs() < 0.0001);
            assert!((after.y - before.y + 20. * dpi).abs() < 0.0001);
            assert_eq!(after.scale, before.scale);
            assert_eq!(after.footer_x, before.footer_x);
            assert_eq!(after.footer_y, before.footer_y);
            let carried = view.framed_point(x + 60. * dpi, y - 20. * dpi, 1920, 1080, vw, vh, dpi);
            assert!((carried.x - point.x).abs() < 0.0001 && (carried.y - point.y).abs() < 0.0001);
            assert!(view.end_pan());
            assert!(!view.drag_pan(x + 400., y));
        }
    }
}

#[test]
fn chrome_and_invalid_input_cannot_start_pan_and_cancellation_or_reset_releases_it() {
    let mut view = EditorView::default();
    for (x, y) in [
        (20., 25.),
        (450., 540.),
        (f64::NAN, 200.),
        (300., f64::INFINITY),
    ] {
        assert!(!view.begin_pan(x, y, 940, 612, 1.));
        assert!(!view.is_panning());
    }
    assert!(view.begin_pan(450., 200., 940, 612, 1.));
    assert!(!view.drag_pan(f64::NAN, 10.));
    assert!(view.end_pan()); // The same cancellation used for focus loss and cursor exit.
    assert!(!view.end_pan());
    assert!(!view.drag_pan(500., 210.));
    assert!(view.begin_pan(450., 200., 940, 612, 1.));
    assert!(view.drag_pan(500., 210.));
    view = EditorView::default(); // Fit / Ctrl+0 resets the view and drag together.
    assert!(!view.is_panning());
    assert_eq!(view.zoom, 1.);
}

#[test]
fn annotation_targets_have_constant_screen_slop_after_pan_zoom_and_dpi_changes() {
    for dpi in [1., 1.5, 2.] {
        let (vw, vh) = ((940. * dpi) as u32, (612. * dpi) as u32);
        for zoom in [0.25, 1., 4.] {
            let mut view = EditorView::default();
            view.zoom = zoom;
            view.begin_pan(450. * dpi, 200. * dpi, vw, vh, dpi);
            view.drag_pan(500. * dpi, 220. * dpi);
            let layout = view.framed_layout(1920, 1080, vw, vh, dpi);
            let slop = view.annotation_hit_slop(1920, 1080, vw, vh, dpi);
            let line = Annotation::Line {
                start: Point::new(100., 100.),
                end: Point::new(1000., 100.),
                color: Color::red(),
                thickness: 1.,
            };
            let near = Point::new(500., 100. + (8. * dpi / layout.scale) as f32);
            let far = Point::new(500., 100. + (11. * dpi / layout.scale) as f32);
            assert!(hit_test_annotation(&line, &near, slop));
            assert!(!hit_test_annotation(&line, &far, slop));
        }
    }
}
