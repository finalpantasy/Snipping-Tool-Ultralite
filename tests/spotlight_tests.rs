use snipping_tool_ul::geometry::{Point, Size};
use snipping_tool_ul::tools::{spotlight::SpotlightTool, Annotation, AnnotationTool};

#[test]
fn spotlight_produces_annotation() {
    let mut tool = SpotlightTool::new();
    tool.on_mouse_down(Point::new(10.0, 10.0));
    tool.on_mouse_move(Point::new(50.0, 50.0));
    let ann = tool.on_mouse_up(Point::new(50.0, 50.0));
    assert!(ann.is_some());
    match ann.unwrap() {
        Annotation::Spotlight { top_left, size } => {
            assert_eq!(top_left, Point::new(10.0, 10.0));
            assert_eq!(size, Size::new(40.0, 40.0));
        }
        _ => panic!("Expected Spotlight annotation"),
    }
}

#[test]
fn spotlight_normalizes_reverse_drag() {
    let mut tool = SpotlightTool::new();
    tool.on_mouse_down(Point::new(50.0, 50.0));
    tool.on_mouse_move(Point::new(10.0, 10.0));
    let ann = tool.on_mouse_up(Point::new(10.0, 10.0));
    assert!(ann.is_some());
    match ann.unwrap() {
        Annotation::Spotlight { top_left, size } => {
            assert_eq!(top_left, Point::new(10.0, 10.0));
            assert_eq!(size, Size::new(40.0, 40.0));
        }
        _ => panic!("Expected Spotlight annotation"),
    }
}

#[test]
fn spotlight_in_progress_annotation() {
    let mut tool = SpotlightTool::new();
    assert!(tool.in_progress_annotation().is_none());
    tool.on_mouse_down(Point::new(10.0, 10.0));
    tool.on_mouse_move(Point::new(40.0, 40.0));
    assert!(tool.is_drawing());
    let preview = tool.in_progress_annotation();
    assert!(preview.is_some());
    match preview.unwrap() {
        Annotation::Spotlight { top_left, size } => {
            assert_eq!(top_left, Point::new(10.0, 10.0));
            assert_eq!(size, Size::new(30.0, 30.0));
        }
        _ => panic!("Expected Spotlight annotation"),
    }
}

#[test]
fn spotlight_no_annotation_without_mouse_down() {
    let mut tool = SpotlightTool::new();
    tool.on_mouse_move(Point::new(50.0, 50.0));
    let ann = tool.on_mouse_up(Point::new(50.0, 50.0));
    assert!(ann.is_none());
}
