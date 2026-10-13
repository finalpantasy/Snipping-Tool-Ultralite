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
        about::draw(&mut frame, false);
        assert!(frame
            .data()
            .chunks_exact(4)
            .all(|p| p[..3].iter().all(|c| *c <= p[3])));
        let r = about::close_rect();
        assert!(r.contains(Point::new(220., 260.)));
        assert!(!r.contains(Point::new(220., 230.)));
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
