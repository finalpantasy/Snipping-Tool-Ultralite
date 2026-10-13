//! Exact comparisons with the pre-optimization renderer; synthetic pixels only.
#[path = "support/text_reference.rs"]
mod reference;

use snipping_tool_ul::geometry::{Color, Point};
use snipping_tool_ul::tools::{render_annotation, render_text_annotation, Annotation};

fn frame() -> tiny_skia::Pixmap {
    let mut p = tiny_skia::Pixmap::new(256, 96).unwrap();
    p.fill(tiny_skia::Color::from_rgba(0.2, 0.3, 0.4, 0.6).unwrap());
    p
}

#[test]
fn clipped_text_matches_previous_pixels_at_each_edge_and_zoom() {
    for size in [8., 31.5, 128., 256.] {
        for position in [
            Point::new(0., 0.),
            Point::new(7.3, 9.8),
            Point::new(-15.5, -4.5),
            Point::new(230., 70.),
            Point::new(400., 10.),
            Point::new(-400., -80.),
        ] {
            for text in ["Hello 123", "éİÅ… Ελληνικά", "A\u{200d}🦀中Ж"] {
                for alpha in [1., 0.4] {
                    let color = Color::new(0.3, 0.8, 0.1, alpha);
                    let mut expected = frame();
                    reference::reference_text(&mut expected, &position, text, &color, size);
                    let mut actual = frame();
                    render_text_annotation(&mut actual, &position, text, &color, size);
                    assert_eq!(
                        actual.data(),
                        expected.data(),
                        "size={size}, pos={position:?}, text={text}"
                    );
                }
            }
        }
    }
}

#[test]
fn numbered_step_metrics_keep_previous_rendered_pixels() {
    for number in [1, 9, 10, 99, 100, u32::MAX] {
        for size in [16., 28., 56., 100.] {
            let annotation = Annotation::StepMarker {
                position: Point::new(128.5, 48.5),
                number,
                color: Color::new(0.1, 0.8, 0.7, 0.7),
                size,
            };
            let mut expected = frame();
            reference::reference_marker(&annotation, &mut expected);
            let mut actual = frame();
            render_annotation(&annotation, &mut actual, None, None);
            assert_eq!(
                actual.data(),
                expected.data(),
                "number={number}, size={size}"
            );
        }
    }
}
