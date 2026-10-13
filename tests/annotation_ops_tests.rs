use snipping_tool_ul::geometry::{Color, Point, Size};
use snipping_tool_ul::tools::{
    annotation_bounding_box, hit_test_annotation, move_annotation, recolor_annotation, Annotation,
};

// ---- Helper constructors ----

fn make_arrow() -> Annotation {
    Annotation::Arrow {
        start: Point::new(10.0, 10.0),
        end: Point::new(110.0, 10.0),
        color: Color::red(),
        thickness: 3.0,
    }
}

fn make_line() -> Annotation {
    Annotation::Line {
        start: Point::new(0.0, 0.0),
        end: Point::new(100.0, 0.0),
        color: Color::blue(),
        thickness: 2.0,
    }
}

fn make_rectangle() -> Annotation {
    Annotation::Rectangle {
        top_left: Point::new(20.0, 20.0),
        size: Size::new(80.0, 60.0),
        color: Color::red(),
        thickness: 3.0,
    }
}

fn make_rounded_rect() -> Annotation {
    Annotation::RoundedRect {
        top_left: Point::new(20.0, 20.0),
        size: Size::new(80.0, 60.0),
        color: Color::red(),
        thickness: 3.0,
        radius: 10.0,
    }
}

fn make_ellipse() -> Annotation {
    Annotation::Ellipse {
        center: Point::new(100.0, 100.0),
        radius_x: 40.0,
        radius_y: 30.0,
        color: Color::red(),
        thickness: 3.0,
    }
}

fn make_highlight() -> Annotation {
    Annotation::Highlight {
        top_left: Point::new(10.0, 10.0),
        size: Size::new(50.0, 30.0),
        color: Color::yellow(),
    }
}

fn make_pixelate() -> Annotation {
    Annotation::Pixelate {
        top_left: Point::new(10.0, 10.0),
        size: Size::new(50.0, 30.0),
        block_size: 8,
    }
}

fn make_pencil() -> Annotation {
    Annotation::Pencil {
        points: vec![
            Point::new(0.0, 0.0),
            Point::new(50.0, 0.0),
            Point::new(50.0, 50.0),
        ],
        color: Color::red(),
        thickness: 2.0,
    }
}

fn make_text() -> Annotation {
    Annotation::Text {
        position: Point::new(30.0, 30.0),
        text: "Hello".to_string(),
        color: Color::red(),
        font_size: 20.0,
    }
}

fn make_step_marker() -> Annotation {
    Annotation::StepMarker {
        position: Point::new(50.0, 50.0),
        number: 1,
        color: Color::red(),
        size: 30.0,
    }
}

// ============================================================
// hit_test_annotation tests
// ============================================================

#[test]
fn hit_test_arrow_near_line() {
    let ann = make_arrow();
    // Point on the line segment (midpoint)
    assert!(hit_test_annotation(&ann, &Point::new(60.0, 10.0), 5.0));
    // Point far away
    assert!(!hit_test_annotation(&ann, &Point::new(60.0, 100.0), 5.0));
}

#[test]
fn hit_test_line_near_segment() {
    let ann = make_line();
    assert!(hit_test_annotation(&ann, &Point::new(50.0, 1.0), 5.0));
    assert!(!hit_test_annotation(&ann, &Point::new(50.0, 50.0), 5.0));
}

#[test]
fn hit_test_rectangle_near_edge() {
    let ann = make_rectangle();
    // Near the top edge
    assert!(hit_test_annotation(&ann, &Point::new(60.0, 20.0), 5.0));
    // Center of the rectangle (far from edges)
    assert!(!hit_test_annotation(&ann, &Point::new(60.0, 50.0), 3.0));
}

#[test]
fn hit_test_rounded_rect_near_edge() {
    let ann = make_rounded_rect();
    // Near the left edge
    assert!(hit_test_annotation(&ann, &Point::new(20.0, 50.0), 5.0));
    // Center (far from edges)
    assert!(!hit_test_annotation(&ann, &Point::new(60.0, 50.0), 3.0));
}

#[test]
fn hit_test_ellipse_near_border() {
    let ann = make_ellipse();
    // Point on the right edge of the ellipse (center.x + radius_x, center.y)
    assert!(hit_test_annotation(&ann, &Point::new(140.0, 100.0), 5.0));
    // Center of ellipse (far from border)
    assert!(!hit_test_annotation(&ann, &Point::new(100.0, 100.0), 3.0));
}

#[test]
fn hit_test_highlight_inside_rect() {
    let ann = make_highlight();
    // Inside
    assert!(hit_test_annotation(&ann, &Point::new(30.0, 25.0), 5.0));
    // Outside
    assert!(!hit_test_annotation(&ann, &Point::new(200.0, 200.0), 5.0));
}

#[test]
fn hit_test_pixelate_inside_rect() {
    let ann = make_pixelate();
    assert!(hit_test_annotation(&ann, &Point::new(30.0, 25.0), 5.0));
    assert!(!hit_test_annotation(&ann, &Point::new(200.0, 200.0), 5.0));
}

#[test]
fn hit_test_pencil_near_segment() {
    let ann = make_pencil();
    // Near the first segment (horizontal at y=0)
    assert!(hit_test_annotation(&ann, &Point::new(25.0, 1.0), 5.0));
    // Far away
    assert!(!hit_test_annotation(&ann, &Point::new(200.0, 200.0), 5.0));
}

#[test]
fn hit_test_text_inside_bbox() {
    let ann = make_text();
    // Inside the text bounding box
    assert!(hit_test_annotation(&ann, &Point::new(35.0, 35.0), 5.0));
    // Outside
    assert!(!hit_test_annotation(&ann, &Point::new(200.0, 200.0), 5.0));
}

#[test]
fn hit_test_step_marker_inside_circle() {
    let ann = make_step_marker();
    // At center
    assert!(hit_test_annotation(&ann, &Point::new(50.0, 50.0), 5.0));
    // Far away
    assert!(!hit_test_annotation(&ann, &Point::new(200.0, 200.0), 5.0));
}

// ============================================================
// move_annotation tests
// ============================================================

#[test]
fn move_arrow() {
    let mut ann = make_arrow();
    move_annotation(&mut ann, 10.0, 20.0);
    match &ann {
        Annotation::Arrow { start, end, .. } => {
            assert_eq!(start.x, 20.0);
            assert_eq!(start.y, 30.0);
            assert_eq!(end.x, 120.0);
            assert_eq!(end.y, 30.0);
        }
        _ => panic!("Expected Arrow"),
    }
}

#[test]
fn move_rectangle() {
    let mut ann = make_rectangle();
    move_annotation(&mut ann, 5.0, -5.0);
    match &ann {
        Annotation::Rectangle { top_left, size, .. } => {
            assert_eq!(top_left.x, 25.0);
            assert_eq!(top_left.y, 15.0);
            // Size should be unchanged
            assert_eq!(size.width, 80.0);
            assert_eq!(size.height, 60.0);
        }
        _ => panic!("Expected Rectangle"),
    }
}

#[test]
fn move_ellipse() {
    let mut ann = make_ellipse();
    move_annotation(&mut ann, 10.0, 20.0);
    match &ann {
        Annotation::Ellipse { center, .. } => {
            assert_eq!(center.x, 110.0);
            assert_eq!(center.y, 120.0);
        }
        _ => panic!("Expected Ellipse"),
    }
}

#[test]
fn move_pencil() {
    let mut ann = make_pencil();
    move_annotation(&mut ann, 10.0, 10.0);
    match &ann {
        Annotation::Pencil { points, .. } => {
            assert_eq!(points[0], Point::new(10.0, 10.0));
            assert_eq!(points[1], Point::new(60.0, 10.0));
            assert_eq!(points[2], Point::new(60.0, 60.0));
        }
        _ => panic!("Expected Pencil"),
    }
}

#[test]
fn move_text() {
    let mut ann = make_text();
    move_annotation(&mut ann, 10.0, 20.0);
    match &ann {
        Annotation::Text { position, .. } => {
            assert_eq!(position.x, 40.0);
            assert_eq!(position.y, 50.0);
        }
        _ => panic!("Expected Text"),
    }
}

#[test]
fn move_step_marker() {
    let mut ann = make_step_marker();
    move_annotation(&mut ann, -10.0, -10.0);
    match &ann {
        Annotation::StepMarker { position, .. } => {
            assert_eq!(position.x, 40.0);
            assert_eq!(position.y, 40.0);
        }
        _ => panic!("Expected StepMarker"),
    }
}

// ============================================================
// recolor_annotation tests
// ============================================================

#[test]
fn recolor_arrow() {
    let mut ann = make_arrow();
    recolor_annotation(&mut ann, Color::blue());
    match &ann {
        Annotation::Arrow { color, .. } => assert_eq!(*color, Color::blue()),
        _ => panic!("Expected Arrow"),
    }
}

#[test]
fn recolor_rectangle() {
    let mut ann = make_rectangle();
    recolor_annotation(&mut ann, Color::green());
    match &ann {
        Annotation::Rectangle { color, .. } => assert_eq!(*color, Color::green()),
        _ => panic!("Expected Rectangle"),
    }
}

#[test]
fn recolor_pixelate_is_noop() {
    let mut ann = make_pixelate();
    let before = ann.clone();
    recolor_annotation(&mut ann, Color::blue());
    assert_eq!(ann, before); // Pixelate has no color, so unchanged
}

#[test]
fn recolor_all_colored_types() {
    let new_color = Color::green();
    let mut types: Vec<Annotation> = vec![
        make_arrow(),
        make_line(),
        make_rectangle(),
        make_rounded_rect(),
        make_ellipse(),
        make_highlight(),
        make_pencil(),
        make_text(),
        make_step_marker(),
    ];
    for ann in types.iter_mut() {
        recolor_annotation(ann, new_color);
    }
    // Verify each got recolored
    for ann in &types {
        let got_color = match ann {
            Annotation::Arrow { color, .. }
            | Annotation::Line { color, .. }
            | Annotation::Rectangle { color, .. }
            | Annotation::RoundedRect { color, .. }
            | Annotation::Ellipse { color, .. }
            | Annotation::Highlight { color, .. }
            | Annotation::Pencil { color, .. }
            | Annotation::Text { color, .. }
            | Annotation::StepMarker { color, .. }
            | Annotation::Measurement { color, .. } => *color,
            Annotation::Pixelate { .. } | Annotation::Spotlight { .. } => unreachable!(),
        };
        assert_eq!(got_color, new_color);
    }
}

// ============================================================
// annotation_bounding_box tests
// ============================================================

#[test]
fn bbox_arrow() {
    let ann = make_arrow();
    let (x, y, w, h) = annotation_bounding_box(&ann).unwrap();
    // Bounds include the 18 px wide head and end exactly at its pointed tip.
    assert!((x - 10.0).abs() < 0.01);
    assert!((y - 1.0).abs() < 0.01);
    assert!((w - 100.0).abs() < 0.01);
    assert!((h - 18.0).abs() < 0.01);
}

#[test]
fn bbox_rectangle() {
    let ann = make_rectangle();
    let (x, y, w, h) = annotation_bounding_box(&ann).unwrap();
    assert_eq!(x, 20.0);
    assert_eq!(y, 20.0);
    assert_eq!(w, 80.0);
    assert_eq!(h, 60.0);
}

#[test]
fn bbox_ellipse() {
    let ann = make_ellipse();
    let (x, y, w, h) = annotation_bounding_box(&ann).unwrap();
    assert_eq!(x, 60.0); // 100 - 40
    assert_eq!(y, 70.0); // 100 - 30
    assert_eq!(w, 80.0); // 40 * 2
    assert_eq!(h, 60.0); // 30 * 2
}

#[test]
fn bbox_step_marker() {
    let ann = make_step_marker();
    let (x, y, w, h) = annotation_bounding_box(&ann).unwrap();
    // position (50,50), size 30 -> half 15
    assert_eq!(x, 35.0);
    assert_eq!(y, 35.0);
    assert_eq!(w, 30.0);
    assert_eq!(h, 30.0);
}

#[test]
fn bbox_text() {
    let ann = make_text();
    let (x, y, w, h) = annotation_bounding_box(&ann).unwrap();
    assert!((27.0..=33.0).contains(&x));
    assert!((30.0..=42.0).contains(&y));
    // Bounds follow the actual glyph ink, rather than the unused font line box.
    // Sanity: a 5-character word at 20px lands in a plausible range.
    assert!(w > 20.0 && w < 120.0, "unexpected text width: {w}");
    assert!((10.0..=30.0).contains(&h), "unexpected text height: {h}");
}

#[test]
fn measure_text_width_grows_with_content() {
    use snipping_tool_ul::tools::measure_text_width;
    let short = measure_text_width("Hi", 20.0);
    let long = measure_text_width("Hi there, world", 20.0);
    assert!(short > 0.0);
    assert!(long > short);
    // Bigger font, wider text
    assert!(measure_text_width("Hi", 40.0) > short);
}

#[test]
fn bbox_pencil() {
    let ann = make_pencil();
    let bbox = annotation_bounding_box(&ann).unwrap();
    // Points: (0,0), (50,0), (50,50), thickness 2 -> half=1
    assert!((bbox.0 - -1.0).abs() < 0.01); // min_x - half
    assert!((bbox.1 - -1.0).abs() < 0.01); // min_y - half
}

#[test]
fn bbox_pencil_empty() {
    let ann = Annotation::Pencil {
        points: vec![],
        color: Color::red(),
        thickness: 2.0,
    };
    assert!(annotation_bounding_box(&ann).is_none());
}
