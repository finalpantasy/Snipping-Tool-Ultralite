use snipping_tool_ul::geometry::{Color, Point};
use snipping_tool_ul::tools::{
    measurement::MeasurementTool, render_annotation, Annotation, AnnotationTool,
};

#[test]
fn measurement_produces_correct_annotation() {
    let color = Color::new(1.0, 0.0, 0.0, 1.0);
    let mut tool = MeasurementTool::new(color);
    tool.on_mouse_down(Point::new(10.0, 20.0));
    tool.on_mouse_move(Point::new(50.0, 60.0));
    let ann = tool.on_mouse_up(Point::new(50.0, 60.0));
    assert!(ann.is_some());
    match ann.unwrap() {
        Annotation::Measurement {
            start,
            end,
            color: c,
        } => {
            assert_eq!(start, Point::new(10.0, 20.0));
            assert_eq!(end, Point::new(50.0, 60.0));
            assert_eq!(c, color);
        }
        _ => panic!("Expected Measurement annotation"),
    }
}

#[test]
fn measurement_distance_horizontal() {
    let color = Color::new(1.0, 0.0, 0.0, 1.0);
    let mut tool = MeasurementTool::new(color);
    tool.on_mouse_down(Point::new(0.0, 50.0));
    tool.on_mouse_move(Point::new(100.0, 50.0));
    let ann = tool.on_mouse_up(Point::new(100.0, 50.0));
    match ann.unwrap() {
        Annotation::Measurement { start, end, .. } => {
            let dx = end.x - start.x;
            let dy = end.y - start.y;
            let distance = (dx * dx + dy * dy).sqrt();
            assert!((distance - 100.0).abs() < 0.01);
        }
        _ => panic!("Expected Measurement annotation"),
    }
}

#[test]
fn measurement_render_smoke_test() {
    let color = Color::new(1.0, 0.0, 0.0, 1.0);
    let ann = Annotation::Measurement {
        start: Point::new(10.0, 10.0),
        end: Point::new(60.0, 10.0),
        color,
    };
    let mut pixmap = tiny_skia::Pixmap::new(200, 200).unwrap();
    render_annotation(&ann, &mut pixmap, None, None);
    // Verify some pixels were drawn (not all zeros)
    let has_drawn = pixmap.data().chunks(4).any(|px| px[3] > 0);
    assert!(
        has_drawn,
        "Measurement annotation should render visible pixels"
    );
}

#[test]
fn measurement_click_without_drag_produces_nothing() {
    // A zero-length measurement is meaningless — clicks must not create one.
    let color = Color::new(1.0, 0.0, 0.0, 1.0);
    let mut tool = MeasurementTool::new(color);
    tool.on_mouse_down(Point::new(50.0, 50.0));
    let ann = tool.on_mouse_up(Point::new(50.0, 50.0));
    assert!(ann.is_none());
}

#[test]
fn measurement_no_annotation_without_mouse_down() {
    let color = Color::new(1.0, 0.0, 0.0, 1.0);
    let mut tool = MeasurementTool::new(color);
    tool.on_mouse_move(Point::new(50.0, 50.0));
    let ann = tool.on_mouse_up(Point::new(50.0, 50.0));
    assert!(ann.is_none());
}

#[test]
fn measurement_in_progress_annotation() {
    let color = Color::new(1.0, 0.0, 0.0, 1.0);
    let mut tool = MeasurementTool::new(color);
    assert!(tool.in_progress_annotation().is_none());
    tool.on_mouse_down(Point::new(10.0, 10.0));
    tool.on_mouse_move(Point::new(40.0, 40.0));
    assert!(tool.is_drawing());
    let preview = tool.in_progress_annotation();
    assert!(preview.is_some());
    match preview.unwrap() {
        Annotation::Measurement { start, end, .. } => {
            assert_eq!(start, Point::new(10.0, 10.0));
            assert_eq!(end, Point::new(40.0, 40.0));
        }
        _ => panic!("Expected Measurement annotation"),
    }
}
