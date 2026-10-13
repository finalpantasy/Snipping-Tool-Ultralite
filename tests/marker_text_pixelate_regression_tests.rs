use snipping_tool_ul::geometry::{Color, Point};
use snipping_tool_ul::tools::{
    annotation_bounding_box, render_annotation, resize_annotation, Annotation, ResizeHandle,
};

fn assert_close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 0.001,
        "expected {expected}, got {actual}"
    );
}

fn opposite_corner(handle: ResizeHandle, x: f32, y: f32, width: f32, height: f32) -> Point {
    match handle {
        ResizeHandle::TopLeft => Point::new(x + width, y + height),
        ResizeHandle::TopRight => Point::new(x, y + height),
        ResizeHandle::BottomLeft => Point::new(x + width, y),
        ResizeHandle::BottomRight => Point::new(x, y),
    }
}

fn drag_past_opposite_corner(
    handle: ResizeHandle,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
) -> Point {
    let anchor = opposite_corner(handle, x, y, width, height);
    match handle {
        ResizeHandle::TopLeft => Point::new(anchor.x + 5.0, anchor.y + 5.0),
        ResizeHandle::TopRight => Point::new(anchor.x - 5.0, anchor.y + 5.0),
        ResizeHandle::BottomLeft => Point::new(anchor.x + 5.0, anchor.y - 5.0),
        ResizeHandle::BottomRight => Point::new(anchor.x - 5.0, anchor.y - 5.0),
    }
}

fn dragged_corner(handle: ResizeHandle, x: f32, y: f32, width: f32, height: f32) -> Point {
    match handle {
        ResizeHandle::TopLeft => Point::new(x, y),
        ResizeHandle::TopRight => Point::new(x + width, y),
        ResizeHandle::BottomLeft => Point::new(x, y + height),
        ResizeHandle::BottomRight => Point::new(x + width, y + height),
    }
}

#[test]
fn aspect_changing_marker_resize_keeps_the_opposite_handle_fixed() {
    let mut annotation = Annotation::StepMarker {
        position: Point::new(100.0, 80.0),
        number: 1,
        color: Color::red(),
        size: 28.0,
    };
    let (x, y, width, height) = annotation_bounding_box(&annotation).unwrap();
    let fixed_right = x + width;
    let fixed_bottom = y + height;

    resize_annotation(
        &mut annotation,
        ResizeHandle::TopLeft,
        Point::new(x - 40.0, y - 10.0),
    );

    let (x, y, width, height) = annotation_bounding_box(&annotation).unwrap();
    assert_close(width, 38.0);
    assert_close(height, 38.0);
    assert_close(x + width, fixed_right);
    assert_close(y + height, fixed_bottom);
}

#[test]
fn marker_resize_clamps_to_the_supported_diameter_and_keeps_its_anchor() {
    let mut annotation = Annotation::StepMarker {
        position: Point::new(100.0, 80.0),
        number: 1,
        color: Color::red(),
        size: 28.0,
    };
    let (x, y, width, height) = annotation_bounding_box(&annotation).unwrap();
    let fixed_right = x + width;
    let fixed_bottom = y + height;

    resize_annotation(
        &mut annotation,
        ResizeHandle::TopLeft,
        Point::new(x - 1000.0, y - 1000.0),
    );

    let (x, y, width, height) = annotation_bounding_box(&annotation).unwrap();
    assert_close(width, 60.0);
    assert_close(height, 60.0);
    assert_close(x + width, fixed_right);
    assert_close(y + height, fixed_bottom);
}

#[test]
fn clamped_text_resize_keeps_the_opposite_ink_corner_fixed() {
    let mut annotation = Annotation::Text {
        position: Point::new(100.0, 80.0),
        text: "Clamp".to_owned(),
        color: Color::red(),
        font_size: 20.0,
    };
    let (x, y, width, height) = annotation_bounding_box(&annotation).unwrap();
    let fixed_right = x + width;
    let fixed_bottom = y + height;

    resize_annotation(
        &mut annotation,
        ResizeHandle::TopLeft,
        Point::new(x - 1000.0, y - 1000.0),
    );

    let (x, y, width, height) = annotation_bounding_box(&annotation).unwrap();
    match &annotation {
        Annotation::Text { font_size, .. } => assert_close(*font_size, 128.0),
        _ => unreachable!(),
    }
    assert_close(x + width, fixed_right);
    assert_close(y + height, fixed_bottom);
}

#[test]
fn marker_resize_below_minimum_clamps_and_keeps_its_anchor_from_all_corners() {
    for handle in [
        ResizeHandle::TopLeft,
        ResizeHandle::TopRight,
        ResizeHandle::BottomLeft,
        ResizeHandle::BottomRight,
    ] {
        let mut annotation = Annotation::StepMarker {
            position: Point::new(100.0, 80.0),
            number: 1,
            color: Color::red(),
            size: 28.0,
        };
        let (x, y, width, height) = annotation_bounding_box(&annotation).unwrap();
        let anchor = opposite_corner(handle, x, y, width, height);
        let past_anchor = drag_past_opposite_corner(handle, x, y, width, height);

        resize_annotation(&mut annotation, handle, past_anchor);

        let (x, y, width, height) = annotation_bounding_box(&annotation).unwrap();
        assert_close(width, 16.0);
        assert_close(height, 16.0);
        let actual_anchor = opposite_corner(handle, x, y, width, height);
        assert_close(actual_anchor.x, anchor.x);
        assert_close(actual_anchor.y, anchor.y);
    }
}

#[test]
fn text_resize_below_minimum_clamps_and_keeps_its_anchor_from_all_corners() {
    for handle in [
        ResizeHandle::TopLeft,
        ResizeHandle::TopRight,
        ResizeHandle::BottomLeft,
        ResizeHandle::BottomRight,
    ] {
        let mut annotation = Annotation::Text {
            position: Point::new(100.0, 80.0),
            text: "Clamp".to_owned(),
            color: Color::red(),
            font_size: 20.0,
        };
        let (x, y, width, height) = annotation_bounding_box(&annotation).unwrap();
        let anchor = opposite_corner(handle, x, y, width, height);
        let past_anchor = drag_past_opposite_corner(handle, x, y, width, height);

        resize_annotation(&mut annotation, handle, past_anchor);

        match &annotation {
            Annotation::Text { font_size, .. } => assert_close(*font_size, 8.0),
            _ => unreachable!(),
        }
        let (x, y, width, height) = annotation_bounding_box(&annotation).unwrap();
        let actual_anchor = opposite_corner(handle, x, y, width, height);
        assert_close(actual_anchor.x, anchor.x);
        assert_close(actual_anchor.y, anchor.y);
    }
}

#[test]
fn endpoint_line_and_arrow_handles_preserve_geometry_when_not_dragged() {
    for handle in [
        ResizeHandle::TopLeft,
        ResizeHandle::TopRight,
        ResizeHandle::BottomLeft,
        ResizeHandle::BottomRight,
    ] {
        let annotations = [
            Annotation::Line {
                start: Point::new(20.0, 40.0),
                end: Point::new(100.0, 80.0),
                color: Color::red(),
                thickness: 8.0,
            },
            Annotation::Arrow {
                start: Point::new(20.0, 40.0),
                end: Point::new(100.0, 80.0),
                color: Color::red(),
                thickness: 8.0,
            },
        ];

        for mut annotation in annotations {
            let (x, y, width, height) =
                snipping_tool_ul::tools::annotation_resize_bounds(&annotation).unwrap();
            let pointer = dragged_corner(handle, x, y, width, height);
            let (start_before, end_before) = match &annotation {
                Annotation::Line { start, end, .. } | Annotation::Arrow { start, end, .. } => {
                    (*start, *end)
                }
                _ => unreachable!(),
            };

            resize_annotation(&mut annotation, handle, pointer);

            let (start_after, end_after) = match &annotation {
                Annotation::Line { start, end, .. } | Annotation::Arrow { start, end, .. } => {
                    (*start, *end)
                }
                _ => unreachable!(),
            };
            assert_close(start_after.x, start_before.x);
            assert_close(start_after.y, start_before.y);
            assert_close(end_after.x, end_before.x);
            assert_close(end_after.y, end_before.y);
        }
    }
}

#[test]
fn pixelate_render_ignores_transparent_rgb_and_preserves_average_alpha() {
    // Straight RGBA input: transparent red must not tint the visible blue pixel.
    let source = [255, 0, 0, 0, 0, 0, 255, 255];
    let annotation = Annotation::Pixelate {
        top_left: Point::new(0.0, 0.0),
        size: snipping_tool_ul::geometry::Size::new(2.0, 1.0),
        block_size: 2,
    };
    let mut pixmap = tiny_skia::Pixmap::new(2, 1).unwrap();

    render_annotation(&annotation, &mut pixmap, Some(&source), Some(2));

    // tiny-skia stores premultiplied RGBA; the average is half-alpha blue.
    assert_eq!(&pixmap.data()[..4], &[0, 0, 127, 127]);
    assert_eq!(&pixmap.data()[4..8], &[0, 0, 127, 127]);
}
