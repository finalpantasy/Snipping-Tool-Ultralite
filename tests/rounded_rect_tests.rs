use snipping_tool_ul::geometry::{Color, Point, Size};
use snipping_tool_ul::tools::{
    render_annotation, rounded_rect::RoundedRectTool, Annotation, AnnotationTool,
};

#[test]
fn rounded_rect_produces_annotation_with_correct_radius() {
    let mut tool = RoundedRectTool::new(Color::red(), 3.0);
    tool.set_radius(15.0);
    tool.on_mouse_down(Point::new(10.0, 10.0));
    tool.on_mouse_move(Point::new(90.0, 70.0));
    let ann = tool.on_mouse_up(Point::new(90.0, 70.0));
    assert!(ann.is_some());
    match ann.unwrap() {
        Annotation::RoundedRect {
            top_left,
            size,
            color,
            thickness,
            radius,
        } => {
            assert_eq!(top_left, Point::new(10.0, 10.0));
            assert_eq!(size, Size::new(80.0, 60.0));
            assert_eq!(color, Color::red());
            assert_eq!(thickness, 3.0);
            assert_eq!(radius, 15.0);
        }
        _ => panic!("Expected RoundedRect annotation"),
    }
}

#[test]
fn rounded_rect_normalizes_reverse_drag() {
    let mut tool = RoundedRectTool::new(Color::red(), 3.0);
    tool.on_mouse_down(Point::new(90.0, 70.0));
    tool.on_mouse_move(Point::new(10.0, 10.0));
    let ann = tool.on_mouse_up(Point::new(10.0, 10.0));
    assert!(ann.is_some());
    match ann.unwrap() {
        Annotation::RoundedRect { top_left, size, .. } => {
            assert_eq!(top_left, Point::new(10.0, 10.0));
            assert_eq!(size, Size::new(80.0, 60.0));
        }
        _ => panic!("Expected RoundedRect annotation"),
    }
}

#[test]
fn set_radius_clamping() {
    let mut tool = RoundedRectTool::new(Color::red(), 3.0);

    tool.set_radius(1.0); // below min of 2.0
    assert_eq!(tool.radius(), 2.0);

    tool.set_radius(100.0); // above max of 50.0
    assert_eq!(tool.radius(), 50.0);

    tool.set_radius(25.0); // within range
    assert_eq!(tool.radius(), 25.0);
}

#[test]
fn rounded_rect_render_smoke_test() {
    let mut pixmap = tiny_skia::Pixmap::new(200, 200).unwrap();
    let ann = Annotation::RoundedRect {
        top_left: Point::new(20.0, 20.0),
        size: Size::new(100.0, 80.0),
        color: Color::red(),
        thickness: 3.0,
        radius: 10.0,
    };
    render_annotation(&ann, &mut pixmap, None, None);
    // Check a pixel on the top edge (middle of top side, past the corner radius)
    let pixel = pixmap.pixel(70, 20).unwrap();
    assert!(
        pixel.alpha() > 0,
        "Expected non-transparent pixel on rounded rect edge"
    );
}
