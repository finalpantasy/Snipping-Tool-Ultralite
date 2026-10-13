use snipping_tool_ul::geometry::{Point, Size};
use snipping_tool_ul::tools::{
    pixelate::PixelateTool, render_annotation, Annotation, AnnotationTool,
};

#[test]
fn pixelate_produces_annotation() {
    let mut tool = PixelateTool::new(10);
    tool.on_mouse_down(Point::new(10.0, 10.0));
    tool.on_mouse_move(Point::new(60.0, 60.0));
    let ann = tool.on_mouse_up(Point::new(60.0, 60.0));
    assert!(ann.is_some());
    match ann.unwrap() {
        Annotation::Pixelate {
            top_left,
            size,
            block_size,
        } => {
            assert_eq!(top_left, Point::new(10.0, 10.0));
            assert_eq!(size, Size::new(50.0, 50.0));
            assert_eq!(block_size, 10);
        }
        _ => panic!("Expected Pixelate annotation"),
    }
}

#[test]
fn pixelate_normalizes_reverse_drag() {
    let mut tool = PixelateTool::new(8);
    tool.on_mouse_down(Point::new(80.0, 80.0));
    tool.on_mouse_move(Point::new(20.0, 20.0));
    let ann = tool.on_mouse_up(Point::new(20.0, 20.0));
    assert!(ann.is_some());
    match ann.unwrap() {
        Annotation::Pixelate { top_left, size, .. } => {
            assert_eq!(top_left, Point::new(20.0, 20.0));
            assert_eq!(size, Size::new(60.0, 60.0));
        }
        _ => panic!("Expected Pixelate annotation"),
    }
}

#[test]
fn pixelate_render_draws_to_pixmap() {
    // Create a 20x20 "screenshot" with known pixel values: all red (255,0,0,255)
    let width: u32 = 20;
    let height: u32 = 20;
    let mut src_pixels = vec![0u8; (width * height * 4) as usize];
    for i in 0..(width * height) as usize {
        src_pixels[i * 4] = 255; // R
        src_pixels[i * 4 + 1] = 0; // G
        src_pixels[i * 4 + 2] = 0; // B
        src_pixels[i * 4 + 3] = 255; // A
    }

    let mut pixmap = tiny_skia::Pixmap::new(width, height).unwrap();

    let ann = Annotation::Pixelate {
        top_left: Point::new(0.0, 0.0),
        size: Size::new(20.0, 20.0),
        block_size: 10,
    };

    render_annotation(&ann, &mut pixmap, Some(&src_pixels), Some(width));

    // After pixelation of a uniform red image, all pixels should still be red-ish
    let pixel = pixmap.pixel(5, 5).unwrap();
    assert!(
        pixel.alpha() > 0,
        "Expected non-transparent pixel after pixelate"
    );
    // The red channel (premultiplied) should be dominant
    assert!(
        pixel.red() > 200,
        "Expected red channel to be high, got {}",
        pixel.red()
    );
    assert!(
        pixel.green() < 10,
        "Expected green channel to be low, got {}",
        pixel.green()
    );
    assert!(
        pixel.blue() < 10,
        "Expected blue channel to be low, got {}",
        pixel.blue()
    );
}

#[test]
fn pixelate_no_annotation_without_mouse_down() {
    let mut tool = PixelateTool::new(10);
    tool.on_mouse_move(Point::new(50.0, 50.0));
    let ann = tool.on_mouse_up(Point::new(50.0, 50.0));
    assert!(ann.is_none());
}

#[test]
fn pixelate_in_progress_annotation_during_drag() {
    let mut tool = PixelateTool::new(10);
    assert!(tool.in_progress_annotation().is_none());
    tool.on_mouse_down(Point::new(10.0, 10.0));
    tool.on_mouse_move(Point::new(40.0, 40.0));
    assert!(tool.is_drawing());
    let preview = tool.in_progress_annotation();
    assert!(preview.is_some());
    match preview.unwrap() {
        Annotation::Pixelate { top_left, size, .. } => {
            assert_eq!(top_left, Point::new(10.0, 10.0));
            assert_eq!(size, Size::new(30.0, 30.0));
        }
        _ => panic!("Expected Pixelate annotation"),
    }
}

#[test]
fn pixelate_skips_render_without_source_pixels() {
    let mut pixmap = tiny_skia::Pixmap::new(100, 100).unwrap();
    let ann = Annotation::Pixelate {
        top_left: Point::new(0.0, 0.0),
        size: Size::new(50.0, 50.0),
        block_size: 10,
    };
    // Render without source pixels — should be a no-op (no crash)
    render_annotation(&ann, &mut pixmap, None, None);
    // Pixmap should remain transparent (default)
    let pixel = pixmap.pixel(25, 25).unwrap();
    assert_eq!(
        pixel.alpha(),
        0,
        "Expected transparent pixel when no source pixels"
    );
}
