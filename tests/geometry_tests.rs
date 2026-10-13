use snipping_tool_ul::geometry::{Color, Point, Size};

#[test]
fn test_point_creation() {
    let p = Point::new(10.0, 20.0);
    assert_eq!(p.x, 10.0);
    assert_eq!(p.y, 20.0);
}

#[test]
fn test_size_creation() {
    let s = Size::new(100.0, 200.0);
    assert_eq!(s.width, 100.0);
    assert_eq!(s.height, 200.0);
}

#[test]
fn test_color_red() {
    let c = Color::red();
    // Saturated annotation red #ff0000
    assert!((c.r - 1.0).abs() < 0.01);
    assert_eq!(c.g, 0.0);
    assert_eq!(c.b, 0.0);
    assert_eq!(c.a, 1.0);
}

#[test]
fn test_color_presets() {
    let colors = Color::presets();
    assert!(colors.len() >= 4);
    assert_eq!(colors[0], Color::red());
}

#[test]
fn test_color_to_tiny_skia() {
    let c = Color::new(1.0, 0.0, 0.0, 1.0);
    let skia_color: tiny_skia::Color = c.into();
    assert_eq!(skia_color.red(), 1.0);
    assert_eq!(skia_color.green(), 0.0);
}
