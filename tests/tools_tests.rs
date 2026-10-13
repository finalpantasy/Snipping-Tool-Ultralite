use snipping_tool_ul::geometry::{Color, Point, Size};
use snipping_tool_ul::tools::{
    arrow::ArrowTool, arrowhead_points, circle::CircleTool, highlight::HighlightTool,
    line::LineTool, rectangle::RectangleTool, render_annotation, Annotation, AnnotationTool,
};

#[test]
fn pixelation_export_preserves_block_averages_and_clips_oversized_regions() {
    let source: Vec<u8> = (0..35).flat_map(|i| [i * 5, i * 3, i, 255]).collect();
    for size in [Size::new(6., 4.), Size::new(1.0e20, 1.0e20)] {
        let mut frame = tiny_skia::Pixmap::new(7, 5).unwrap();
        let ann = Annotation::Pixelate {
            top_left: Point::new(1., 1.),
            size,
            block_size: 3,
        };
        render_annotation(&ann, &mut frame, Some(&source), Some(7));
        for y in 0..5usize {
            for x in 0..7usize {
                let actual = &frame.data()[(y * 7 + x) * 4..(y * 7 + x + 1) * 4];
                if x == 0 || y == 0 {
                    assert_eq!(actual, &[0; 4]);
                    continue;
                }
                let bx = 1 + ((x - 1) / 3) * 3;
                let by = 1 + ((y - 1) / 3) * 3;
                let mut sum = [0u32; 3];
                let mut count = 0;
                for sy in by..(by + 3).min(5) {
                    for sx in bx..(bx + 3).min(7) {
                        for c in 0..3 {
                            sum[c] += source[(sy * 7 + sx) * 4 + c] as u32;
                        }
                        count += 1;
                    }
                }
                assert_eq!(
                    actual,
                    &[
                        (sum[0] / count) as u8,
                        (sum[1] / count) as u8,
                        (sum[2] / count) as u8,
                        255
                    ]
                );
            }
        }
    }
}

#[test]
fn rectangle_produces_annotation() {
    let mut tool = RectangleTool::new(Color::red(), 3.0);
    tool.on_mouse_down(Point::new(10.0, 10.0));
    tool.on_mouse_move(Point::new(50.0, 50.0));
    let ann = tool.on_mouse_up(Point::new(50.0, 50.0));
    assert!(ann.is_some());
    match ann.unwrap() {
        Annotation::Rectangle {
            top_left,
            size,
            color,
            thickness,
        } => {
            assert_eq!(top_left, Point::new(10.0, 10.0));
            assert_eq!(size, Size::new(40.0, 40.0));
            assert_eq!(color, Color::red());
            assert_eq!(thickness, 3.0);
        }
        _ => panic!("Expected Rectangle annotation"),
    }
}

#[test]
fn rectangle_normalizes_reverse_drag() {
    let mut tool = RectangleTool::new(Color::red(), 3.0);
    tool.on_mouse_down(Point::new(50.0, 50.0));
    tool.on_mouse_move(Point::new(10.0, 10.0));
    let ann = tool.on_mouse_up(Point::new(10.0, 10.0));
    assert!(ann.is_some());
    match ann.unwrap() {
        Annotation::Rectangle { top_left, size, .. } => {
            assert_eq!(top_left, Point::new(10.0, 10.0));
            assert_eq!(size, Size::new(40.0, 40.0));
        }
        _ => panic!("Expected Rectangle annotation"),
    }
}

#[test]
fn arrow_produces_annotation() {
    let mut tool = ArrowTool::new(Color::blue(), 2.0);
    tool.on_mouse_down(Point::new(5.0, 5.0));
    tool.on_mouse_move(Point::new(100.0, 100.0));
    let ann = tool.on_mouse_up(Point::new(100.0, 100.0));
    assert!(ann.is_some());
    match ann.unwrap() {
        Annotation::Arrow {
            start,
            end,
            color,
            thickness,
        } => {
            assert_eq!(start, Point::new(5.0, 5.0));
            assert_eq!(end, Point::new(100.0, 100.0));
            assert_eq!(color, Color::blue());
            assert_eq!(thickness, 2.0);
        }
        _ => panic!("Expected Arrow annotation"),
    }
}

#[test]
fn no_annotation_without_mouse_down() {
    let mut tool = RectangleTool::new(Color::red(), 3.0);
    tool.on_mouse_move(Point::new(50.0, 50.0));
    let ann = tool.on_mouse_up(Point::new(50.0, 50.0));
    assert!(ann.is_none());
}

#[test]
fn in_progress_annotation_works_during_drag() {
    let mut tool = RectangleTool::new(Color::red(), 3.0);
    assert!(tool.in_progress_annotation().is_none());
    tool.on_mouse_down(Point::new(10.0, 10.0));
    tool.on_mouse_move(Point::new(40.0, 40.0));
    assert!(tool.is_drawing());
    let preview = tool.in_progress_annotation();
    assert!(preview.is_some());
    match preview.unwrap() {
        Annotation::Rectangle { top_left, size, .. } => {
            assert_eq!(top_left, Point::new(10.0, 10.0));
            assert_eq!(size, Size::new(30.0, 30.0));
        }
        _ => panic!("Expected Rectangle annotation"),
    }
}

#[test]
fn thickness_clamping() {
    let mut tool = RectangleTool::new(Color::red(), 3.0);
    tool.set_thickness(0.0);
    // Thickness should be clamped to 1.0
    tool.on_mouse_down(Point::new(0.0, 0.0));
    tool.on_mouse_move(Point::new(10.0, 10.0));
    let ann = tool.on_mouse_up(Point::new(10.0, 10.0));
    match ann.unwrap() {
        Annotation::Rectangle { thickness, .. } => assert_eq!(thickness, 1.0),
        _ => panic!("Expected Rectangle"),
    }

    tool.set_thickness(50.0);
    tool.on_mouse_down(Point::new(0.0, 0.0));
    tool.on_mouse_move(Point::new(10.0, 10.0));
    let ann = tool.on_mouse_up(Point::new(10.0, 10.0));
    match ann.unwrap() {
        Annotation::Rectangle { thickness, .. } => assert_eq!(thickness, 20.0),
        _ => panic!("Expected Rectangle"),
    }
}

#[test]
fn arrowhead_points_geometry() {
    let start = Point::new(0.0, 0.0);
    let end = Point::new(100.0, 0.0);
    let thickness = 3.0;
    let points = arrowhead_points(start, end, thickness);
    // Should return 3 points forming a triangle
    assert_eq!(points.len(), 3);
    // Tip should be at the end point
    assert_eq!(points[0], end);
    // The other two points should be behind the tip and symmetric about the shaft
    // Broad head: 12 px long and 18 px wide for a 3 px stroke.
    let back_dist = 12.0_f32;
    let perp_offset = 9.0_f32;
    assert!((points[1].x - (100.0 - back_dist)).abs() < 0.01);
    assert!((points[1].y - perp_offset).abs() < 0.01);
    assert!((points[2].x - (100.0 - back_dist)).abs() < 0.01);
    assert!((points[2].y - (-perp_offset)).abs() < 0.01);
}

#[test]
fn render_annotation_draws_to_pixmap() {
    let mut pixmap = tiny_skia::Pixmap::new(200, 200).unwrap();
    let ann = Annotation::Rectangle {
        top_left: Point::new(10.0, 10.0),
        size: Size::new(50.0, 50.0),
        color: Color::red(),
        thickness: 3.0,
    };
    render_annotation(&ann, &mut pixmap, None, None);
    // Check that a pixel on the rectangle border is non-transparent
    // Top edge at approximately (30, 10)
    let pixel = pixmap.pixel(30, 10).unwrap();
    assert!(
        pixel.alpha() > 0,
        "Expected non-transparent pixel on rectangle edge"
    );

    // Also test arrow rendering
    let mut pixmap2 = tiny_skia::Pixmap::new(200, 200).unwrap();
    let ann2 = Annotation::Arrow {
        start: Point::new(10.0, 100.0),
        end: Point::new(190.0, 100.0),
        color: Color::blue(),
        thickness: 3.0,
    };
    render_annotation(&ann2, &mut pixmap2, None, None);
    // Check pixel along the arrow shaft
    let pixel2 = pixmap2.pixel(100, 100).unwrap();
    assert!(
        pixel2.alpha() > 0,
        "Expected non-transparent pixel on arrow shaft"
    );
}

// ---- Circle/Ellipse tool tests ----

#[test]
fn circle_produces_annotation() {
    let mut tool = CircleTool::new(Color::red(), 3.0);
    tool.on_mouse_down(Point::new(10.0, 20.0));
    tool.on_mouse_move(Point::new(50.0, 60.0));
    let ann = tool.on_mouse_up(Point::new(50.0, 60.0));
    assert!(ann.is_some());
    match ann.unwrap() {
        Annotation::Ellipse {
            center,
            radius_x,
            radius_y,
            color,
            thickness,
        } => {
            assert_eq!(center, Point::new(30.0, 40.0));
            assert_eq!(radius_x, 20.0);
            assert_eq!(radius_y, 20.0);
            assert_eq!(color, Color::red());
            assert_eq!(thickness, 3.0);
        }
        _ => panic!("Expected Ellipse annotation"),
    }
}

#[test]
fn circle_render_smoke_test() {
    let mut pixmap = tiny_skia::Pixmap::new(200, 200).unwrap();
    let ann = Annotation::Ellipse {
        center: Point::new(100.0, 100.0),
        radius_x: 40.0,
        radius_y: 30.0,
        color: Color::red(),
        thickness: 3.0,
    };
    render_annotation(&ann, &mut pixmap, None, None);
    // Check a pixel on the rightmost edge of the ellipse (center.x + radius_x, center.y)
    let pixel = pixmap.pixel(140, 100).unwrap();
    assert!(
        pixel.alpha() > 0,
        "Expected non-transparent pixel on ellipse edge"
    );
}

// ---- Line tool tests ----

#[test]
fn line_produces_annotation() {
    let mut tool = LineTool::new(Color::blue(), 2.0);
    tool.on_mouse_down(Point::new(5.0, 5.0));
    tool.on_mouse_move(Point::new(100.0, 100.0));
    let ann = tool.on_mouse_up(Point::new(100.0, 100.0));
    assert!(ann.is_some());
    match ann.unwrap() {
        Annotation::Line {
            start,
            end,
            color,
            thickness,
        } => {
            assert_eq!(start, Point::new(5.0, 5.0));
            assert_eq!(end, Point::new(100.0, 100.0));
            assert_eq!(color, Color::blue());
            assert_eq!(thickness, 2.0);
        }
        _ => panic!("Expected Line annotation"),
    }
}

#[test]
fn line_render_smoke_test() {
    let mut pixmap = tiny_skia::Pixmap::new(200, 200).unwrap();
    let ann = Annotation::Line {
        start: Point::new(10.0, 100.0),
        end: Point::new(190.0, 100.0),
        color: Color::blue(),
        thickness: 3.0,
    };
    render_annotation(&ann, &mut pixmap, None, None);
    // Check pixel along the line
    let pixel = pixmap.pixel(100, 100).unwrap();
    assert!(pixel.alpha() > 0, "Expected non-transparent pixel on line");
}

// ---- Highlight tool tests ----

#[test]
fn highlight_produces_annotation() {
    let mut tool = HighlightTool::new(Color::yellow());
    tool.on_mouse_down(Point::new(10.0, 10.0));
    tool.on_mouse_move(Point::new(60.0, 40.0));
    let ann = tool.on_mouse_up(Point::new(60.0, 40.0));
    assert!(ann.is_some());
    match ann.unwrap() {
        Annotation::Highlight {
            top_left,
            size,
            color,
        } => {
            assert_eq!(top_left, Point::new(10.0, 10.0));
            assert_eq!(size, Size::new(50.0, 30.0));
            assert_eq!(color, Color::yellow());
        }
        _ => panic!("Expected Highlight annotation"),
    }
}

#[test]
fn highlight_render_smoke_test() {
    let mut pixmap = tiny_skia::Pixmap::new(200, 200).unwrap();
    let ann = Annotation::Highlight {
        top_left: Point::new(20.0, 20.0),
        size: Size::new(80.0, 40.0),
        color: Color::yellow(),
    };
    render_annotation(&ann, &mut pixmap, None, None);
    // Check a pixel inside the highlighted area — should be non-transparent
    let pixel = pixmap.pixel(50, 35).unwrap();
    assert!(
        pixel.alpha() > 0,
        "Expected non-transparent pixel inside highlight"
    );
}
