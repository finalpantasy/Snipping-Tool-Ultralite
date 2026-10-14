use snipping_tool_ul::{about, geometry::Point};

#[test]
fn about_version_and_close_target_remain_correct_at_each_dpi() {
    assert_eq!(about::VERSION, env!("CARGO_PKG_VERSION"));
    for dpi in [1., 1.25, 1.5, 2.] {
        let mut frame = tiny_skia::Pixmap::new(
            (about::WIDTH as f32 * dpi) as u32,
            (about::HEIGHT as f32 * dpi) as u32,
        )
        .unwrap();
        about::draw(&mut frame, false, false);
        assert!(frame
            .data()
            .chunks_exact(4)
            .all(|p| p[..3].iter().all(|c| *c <= p[3])));
        let r = about::close_rect();
        assert!(r.contains(Point::new(306., 260.)));
        assert!(!r.contains(Point::new(306., 230.)));
        assert!(!r.contains(Point::new(220., 260.)), "gap between buttons");
        // The title must retain raster detail at physical resolution, not a blank card.
        let top = (94. * dpi) as usize;
        let bottom = (123. * dpi) as usize;
        let width = frame.width() as usize;
        let colors: std::collections::HashSet<_> = frame.data()
            [top * width * 4..bottom * width * 4]
            .chunks_exact(4)
            .collect();
        assert!(colors.len() > 20, "Missing or flattened title at {dpi} DPI");
    }
}

#[test]
fn support_button_sits_beside_close_as_one_centred_row() {
    let (s, c) = (about::support_rect(), about::close_rect());
    assert_eq!(about::SUPPORT_URL, "https://ko-fi.com/finalpantasy");
    assert!(s.contains(Point::new(158., 260.)));
    assert!(!s.contains(Point::new(306., 260.)));
    assert_eq!((s.y, s.h), (c.y, c.h), "buttons share one row");
    assert_eq!(c.x - (s.x + s.w), 12., "12 px gap");
    assert_eq!(s.x, about::WIDTH as f32 - (c.x + c.w), "row centred");
    assert!(c.y + c.h <= about::HEIGHT as f32 - 16.);
}
