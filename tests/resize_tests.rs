use snipping_tool_ul::geometry::{Color, Point, Size};
use snipping_tool_ul::tools::{
    annotation_bounding_box, resize_annotation, Annotation, ResizeHandle,
};

#[test]
fn resize_rectangle_bottom_right_outward() {
    let mut ann = Annotation::Rectangle {
        top_left: Point::new(20.0, 20.0),
        size: Size::new(80.0, 60.0),
        color: Color::red(),
        thickness: 3.0,
    };
    // Drag bottom-right corner from (100, 80) to (120, 100)
    resize_annotation(
        &mut ann,
        ResizeHandle::BottomRight,
        Point::new(120.0, 100.0),
    );
    match &ann {
        Annotation::Rectangle { top_left, size, .. } => {
            assert_eq!(top_left.x, 20.0);
            assert_eq!(top_left.y, 20.0);
            assert_eq!(size.width, 100.0);
            assert_eq!(size.height, 80.0);
        }
        _ => panic!("Expected Rectangle"),
    }
}

#[test]
fn resize_rectangle_top_left_inward() {
    let mut ann = Annotation::Rectangle {
        top_left: Point::new(20.0, 20.0),
        size: Size::new(80.0, 60.0),
        color: Color::red(),
        thickness: 3.0,
    };
    // Drag top-left corner from (20, 20) to (30, 30)
    resize_annotation(&mut ann, ResizeHandle::TopLeft, Point::new(30.0, 30.0));
    match &ann {
        Annotation::Rectangle { top_left, size, .. } => {
            assert_eq!(top_left.x, 30.0);
            assert_eq!(top_left.y, 30.0);
            assert_eq!(size.width, 70.0);
            assert_eq!(size.height, 50.0);
        }
        _ => panic!("Expected Rectangle"),
    }
}

#[test]
fn resize_arrow_scales_endpoints() {
    let mut ann = Annotation::Arrow {
        start: Point::new(10.0, 10.0),
        end: Point::new(110.0, 60.0),
        color: Color::red(),
        thickness: 3.0,
    };

    let (bx, by, bw, bh) = annotation_bounding_box(&ann).unwrap();

    // Drag bottom-right corner outward, doubling the width
    let new_pos = Point::new(bx + bw * 2.0, by + bh);
    resize_annotation(&mut ann, ResizeHandle::BottomRight, new_pos);

    // The bounding box should now be wider
    let (_, _, new_w, _) = annotation_bounding_box(&ann).unwrap();
    assert!(new_w > bw * 1.5, "Arrow should have scaled wider");
}

#[test]
fn resize_ellipse_changes_radii() {
    let mut ann = Annotation::Ellipse {
        center: Point::new(100.0, 100.0),
        radius_x: 40.0,
        radius_y: 30.0,
        color: Color::red(),
        thickness: 3.0,
    };

    // Drag bottom-right corner outward
    resize_annotation(
        &mut ann,
        ResizeHandle::BottomRight,
        Point::new(160.0, 150.0),
    );

    match &ann {
        Annotation::Ellipse {
            radius_x, radius_y, ..
        } => {
            // New bbox: x=60, y=70, w=100 (was 80), h=80 (was 60)
            assert!(
                *radius_x > 40.0,
                "radius_x should increase, got {}",
                radius_x
            );
            assert!(
                *radius_y > 30.0,
                "radius_y should increase, got {}",
                radius_y
            );
        }
        _ => panic!("Expected Ellipse"),
    }
}

#[test]
fn resize_minimum_size_rejected() {
    let mut ann = Annotation::Rectangle {
        top_left: Point::new(20.0, 20.0),
        size: Size::new(80.0, 60.0),
        color: Color::red(),
        thickness: 3.0,
    };
    let before = ann.clone();

    // Try to make it too small (width < 4.0)
    resize_annotation(&mut ann, ResizeHandle::BottomRight, Point::new(22.0, 22.0));
    // Should be rejected — annotation unchanged
    assert_eq!(ann, before);
}

#[test]
fn resize_rounded_rect() {
    let mut ann = Annotation::RoundedRect {
        top_left: Point::new(10.0, 10.0),
        size: Size::new(60.0, 40.0),
        color: Color::red(),
        thickness: 3.0,
        radius: 8.0,
    };

    resize_annotation(&mut ann, ResizeHandle::BottomRight, Point::new(100.0, 80.0));

    match &ann {
        Annotation::RoundedRect { top_left, size, .. } => {
            assert_eq!(top_left.x, 10.0);
            assert_eq!(top_left.y, 10.0);
            assert_eq!(size.width, 90.0);
            assert_eq!(size.height, 70.0);
        }
        _ => panic!("Expected RoundedRect"),
    }
}
