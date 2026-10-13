use snipping_tool_ul::geometry::{Color, Point};
use snipping_tool_ul::tools::text::TextTool;
use snipping_tool_ul::tools::{render_annotation, Annotation, AnnotationTool};

#[test]
fn text_tool_pending_position_set_on_mouse_down() {
    let mut tool = TextTool::new(Color::red(), 20.0);
    assert!(tool.pending_position().is_none());
    tool.on_mouse_down(Point::new(50.0, 80.0));
    assert_eq!(tool.pending_position(), Some(Point::new(50.0, 80.0)));
}

#[test]
fn text_tool_take_pending_position_consumes() {
    let mut tool = TextTool::new(Color::red(), 20.0);
    tool.on_mouse_down(Point::new(10.0, 20.0));
    let pos = tool.take_pending_position();
    assert_eq!(pos, Some(Point::new(10.0, 20.0)));
    // After take, it should be None
    assert!(tool.pending_position().is_none());
    assert!(tool.take_pending_position().is_none());
}

#[test]
fn text_tool_mouse_up_produces_no_annotation() {
    let mut tool = TextTool::new(Color::blue(), 16.0);
    tool.on_mouse_down(Point::new(10.0, 10.0));
    let ann = tool.on_mouse_up(Point::new(10.0, 10.0));
    assert!(
        ann.is_none(),
        "TextTool should not produce annotations from mouse_up"
    );
}

#[test]
fn text_tool_in_progress_is_none() {
    let mut tool = TextTool::new(Color::green(), 24.0);
    assert!(tool.in_progress_annotation().is_none());
    tool.on_mouse_down(Point::new(30.0, 30.0));
    tool.on_mouse_move(Point::new(60.0, 60.0));
    assert!(
        tool.in_progress_annotation().is_none(),
        "TextTool should never have an in-progress annotation"
    );
}

#[test]
fn text_annotation_renders_visible_pixels() {
    let mut pixmap = tiny_skia::Pixmap::new(200, 200).unwrap();
    let ann = Annotation::Text {
        position: Point::new(10.0, 100.0),
        text: "Hello".to_string(),
        color: Color::red(),
        font_size: 32.0,
    };
    render_annotation(&ann, &mut pixmap, None, None);

    // At least some pixels should be non-transparent after rendering text
    let has_visible = pixmap.pixels().iter().any(|px| px.alpha() > 0);
    assert!(has_visible, "Text annotation should render visible pixels");
}
