use snipping_tool_ul::geometry::{Color, Point};
use snipping_tool_ul::tools::{pencil::PencilTool, render_annotation, Annotation, AnnotationTool};

#[test]
fn pencil_produces_annotation_with_correct_points() {
    let mut tool = PencilTool::new(Color::red(), 2.0);
    tool.on_mouse_down(Point::new(10.0, 10.0));
    tool.on_mouse_move(Point::new(20.0, 20.0));
    tool.on_mouse_move(Point::new(30.0, 15.0));
    let ann = tool.on_mouse_up(Point::new(40.0, 25.0));
    assert!(ann.is_some());
    match ann.unwrap() {
        Annotation::Pencil {
            points,
            color,
            thickness,
        } => {
            assert_eq!(
                points,
                vec![
                    Point::new(10.0, 10.0),
                    Point::new(20.0, 20.0),
                    Point::new(30.0, 15.0),
                    Point::new(40.0, 25.0),
                ]
            );
            assert_eq!(color, Color::red());
            assert_eq!(thickness, 2.0);
        }
        _ => panic!("Expected Pencil annotation"),
    }
}

#[test]
fn no_annotation_without_mouse_down() {
    let mut tool = PencilTool::new(Color::red(), 2.0);
    tool.on_mouse_move(Point::new(20.0, 20.0));
    let ann = tool.on_mouse_up(Point::new(30.0, 30.0));
    assert!(ann.is_none());
}

#[test]
fn in_progress_works_during_drag() {
    let mut tool = PencilTool::new(Color::blue(), 3.0);
    assert!(tool.in_progress_annotation().is_none());
    tool.on_mouse_down(Point::new(5.0, 5.0));
    tool.on_mouse_move(Point::new(15.0, 15.0));
    assert!(tool.is_drawing());
    let preview = tool.in_progress_annotation();
    assert!(preview.is_some());
    match preview.unwrap() {
        Annotation::Pencil {
            points,
            color,
            thickness,
        } => {
            assert_eq!(points.len(), 2);
            assert_eq!(points[0], Point::new(5.0, 5.0));
            assert_eq!(points[1], Point::new(15.0, 15.0));
            assert_eq!(color, Color::blue());
            assert_eq!(thickness, 3.0);
        }
        _ => panic!("Expected Pencil annotation"),
    }
}

#[test]
fn render_annotation_draws_pencil_to_pixmap() {
    let mut pixmap = tiny_skia::Pixmap::new(200, 200).unwrap();
    let ann = Annotation::Pencil {
        points: vec![
            Point::new(10.0, 100.0),
            Point::new(100.0, 100.0),
            Point::new(190.0, 100.0),
        ],
        color: Color::red(),
        thickness: 3.0,
    };
    render_annotation(&ann, &mut pixmap, None, None);
    // Check that a pixel along the pencil stroke is non-transparent
    let pixel = pixmap.pixel(50, 100).unwrap();
    assert!(
        pixel.alpha() > 0,
        "Expected non-transparent pixel on pencil stroke"
    );
}
