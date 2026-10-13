use snipping_tool_ul::geometry::Point;
use snipping_tool_ul::overlay::selection::{HitZone, Selection};
use snipping_tool_ul::overlay::toolbar::Toolbar;

#[test]
fn from_points_creates_correct_selection() {
    let sel = Selection::from_points(Point::new(10.0, 20.0), Point::new(110.0, 120.0));
    assert_eq!(sel.x, 10.0);
    assert_eq!(sel.y, 20.0);
    assert_eq!(sel.width, 100.0);
    assert_eq!(sel.height, 100.0);
}

#[test]
fn from_points_normalizes_reverse_drag() {
    let sel = Selection::from_points(Point::new(110.0, 120.0), Point::new(10.0, 20.0));
    assert_eq!(sel.x, 10.0);
    assert_eq!(sel.y, 20.0);
    assert_eq!(sel.width, 100.0);
    assert_eq!(sel.height, 100.0);
}

#[test]
fn hit_test_returns_inside_for_interior_point() {
    let sel = Selection {
        x: 100.0,
        y: 100.0,
        width: 200.0,
        height: 200.0,
    };
    let result = sel.hit_test(Point::new(200.0, 200.0), 8.0);
    assert_eq!(result, Some(HitZone::Inside));
}

#[test]
fn hit_test_returns_none_for_exterior_point() {
    let sel = Selection {
        x: 100.0,
        y: 100.0,
        width: 200.0,
        height: 200.0,
    };
    let result = sel.hit_test(Point::new(50.0, 50.0), 8.0);
    assert_eq!(result, None);
}

#[test]
fn hit_test_returns_top_left_for_corner_point() {
    let sel = Selection {
        x: 100.0,
        y: 100.0,
        width: 200.0,
        height: 200.0,
    };
    // Point right at the top-left corner
    let result = sel.hit_test(Point::new(100.0, 100.0), 8.0);
    assert_eq!(result, Some(HitZone::TopLeft));
}

#[test]
fn hit_test_returns_top_right_for_corner_point() {
    let sel = Selection {
        x: 100.0,
        y: 100.0,
        width: 200.0,
        height: 200.0,
    };
    let result = sel.hit_test(Point::new(300.0, 100.0), 8.0);
    assert_eq!(result, Some(HitZone::TopRight));
}

#[test]
fn hit_test_returns_bottom_left_for_corner_point() {
    let sel = Selection {
        x: 100.0,
        y: 100.0,
        width: 200.0,
        height: 200.0,
    };
    let result = sel.hit_test(Point::new(100.0, 300.0), 8.0);
    assert_eq!(result, Some(HitZone::BottomLeft));
}

#[test]
fn hit_test_returns_bottom_right_for_corner_point() {
    let sel = Selection {
        x: 100.0,
        y: 100.0,
        width: 200.0,
        height: 200.0,
    };
    let result = sel.hit_test(Point::new(300.0, 300.0), 8.0);
    assert_eq!(result, Some(HitZone::BottomRight));
}

#[test]
fn hit_test_returns_edge_zones() {
    let sel = Selection {
        x: 100.0,
        y: 100.0,
        width: 200.0,
        height: 200.0,
    };

    // Top edge (middle)
    assert_eq!(
        sel.hit_test(Point::new(200.0, 100.0), 8.0),
        Some(HitZone::Top)
    );
    // Bottom edge (middle)
    assert_eq!(
        sel.hit_test(Point::new(200.0, 300.0), 8.0),
        Some(HitZone::Bottom)
    );
    // Left edge (middle)
    assert_eq!(
        sel.hit_test(Point::new(100.0, 200.0), 8.0),
        Some(HitZone::Left)
    );
    // Right edge (middle)
    assert_eq!(
        sel.hit_test(Point::new(300.0, 200.0), 8.0),
        Some(HitZone::Right)
    );
}

#[test]
fn move_by_works_correctly() {
    let mut sel = Selection {
        x: 100.0,
        y: 100.0,
        width: 200.0,
        height: 200.0,
    };
    sel.move_by(10.0, -5.0);
    assert_eq!(sel.x, 110.0);
    assert_eq!(sel.y, 95.0);
    assert_eq!(sel.width, 200.0);
    assert_eq!(sel.height, 200.0);
}

#[test]
fn contains_returns_true_for_interior() {
    let sel = Selection {
        x: 10.0,
        y: 10.0,
        width: 100.0,
        height: 100.0,
    };
    assert!(sel.contains(Point::new(50.0, 50.0)));
}

#[test]
fn contains_returns_false_for_exterior() {
    let sel = Selection {
        x: 10.0,
        y: 10.0,
        width: 100.0,
        height: 100.0,
    };
    assert!(!sel.contains(Point::new(5.0, 5.0)));
}

#[test]
fn resize_top_left_adjusts_origin_and_size() {
    let mut sel = Selection {
        x: 100.0,
        y: 100.0,
        width: 200.0,
        height: 200.0,
    };
    sel.resize(HitZone::TopLeft, 10.0, 5.0);
    assert_eq!(sel.x, 110.0);
    assert_eq!(sel.y, 105.0);
    assert_eq!(sel.width, 190.0);
    assert_eq!(sel.height, 195.0);
}

#[test]
fn resize_bottom_right_adjusts_size_only() {
    let mut sel = Selection {
        x: 100.0,
        y: 100.0,
        width: 200.0,
        height: 200.0,
    };
    sel.resize(HitZone::BottomRight, 10.0, 5.0);
    assert_eq!(sel.x, 100.0);
    assert_eq!(sel.y, 100.0);
    assert_eq!(sel.width, 210.0);
    assert_eq!(sel.height, 205.0);
}

#[test]
fn toolbar_position_below_selection() {
    let sel = Selection {
        x: 100.0,
        y: 100.0,
        width: 400.0,
        height: 200.0,
    };
    let toolbar = Toolbar::position_for(&sel, 1920.0, 1080.0);
    // Should be centered below selection
    let expected_width = Toolbar::toolbar_width();
    let expected_x = (sel.x + (sel.width - expected_width) / 2.0).max(0.0);
    assert_eq!(toolbar.x, expected_x);
    assert_eq!(toolbar.y, sel.y + sel.height + 8.0); // TOOLBAR_PADDING below
}

#[test]
fn toolbar_flips_above_when_near_bottom() {
    let sel = Selection {
        x: 100.0,
        y: 1000.0,
        width: 400.0,
        height: 50.0,
    };
    let toolbar = Toolbar::position_for(&sel, 1920.0, 1080.0);
    // Should be above selection since bottom is near screen edge
    assert!(toolbar.y < sel.y);
}

#[test]
fn toolbar_hit_test_returns_button_index() {
    let sel = Selection {
        x: 100.0,
        y: 100.0,
        width: 400.0,
        height: 200.0,
    };
    let toolbar = Toolbar::position_for(&sel, 1920.0, 1080.0);
    let (bx, by, bw, bh) = toolbar.button_rect(0);
    // Click center of first button
    let result = toolbar.hit_test(Point::new(bx + bw / 2.0, by + bh / 2.0));
    assert_eq!(result, Some(0));
}

#[test]
fn toolbar_hit_test_returns_none_outside() {
    let sel = Selection {
        x: 100.0,
        y: 100.0,
        width: 400.0,
        height: 200.0,
    };
    let toolbar = Toolbar::position_for(&sel, 1920.0, 1080.0);
    let result = toolbar.hit_test(Point::new(0.0, 0.0));
    assert_eq!(result, None);
}
