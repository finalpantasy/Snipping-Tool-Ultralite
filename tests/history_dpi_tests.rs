use snipping_tool_ul::history_ui::{draw_empty_history, WIN_H, WIN_W};
use tiny_skia::Pixmap;

#[test]
fn history_labels_have_native_detail_at_fractional_and_integer_dpi() {
    let mut base = Pixmap::new(WIN_W, WIN_H).unwrap();
    draw_empty_history(&mut base, false);
    for dpi in [1.25, 1.5, 2.] {
        let w = (WIN_W as f32 * dpi) as u32;
        let mut high = Pixmap::new(w, (WIN_H as f32 * dpi) as u32).unwrap();
        draw_empty_history(&mut high, false);
        // Check both the header and empty-state title, excluding other controls.
        for (top, bottom) in [(10., 38.), (85., 117.)] {
            let mut changed = 0;
            for y in (top * dpi) as usize..(bottom * dpi) as usize {
                for x in (16. * dpi) as usize..(210. * dpi) as usize {
                    let a = (y * w as usize + x) * 4;
                    let b = ((y as f32 / dpi) as usize * WIN_W as usize
                        + (x as f32 / dpi) as usize)
                        * 4;
                    changed += usize::from(high.data()[a..a + 4] != base.data()[b..b + 4]);
                }
            }
            assert!(
                changed > 100,
                "Labels must be rasterized at {dpi}x, not enlarged"
            );
        }
    }
}

#[test]
fn history_empty_state_explains_whether_recording_is_enabled() {
    let mut off = Pixmap::new(WIN_W, WIN_H).unwrap();
    let mut on = off.clone();
    draw_empty_history(&mut off, false);
    draw_empty_history(&mut on, true);
    // Header is stable; body changes with the real recording setting.
    assert_eq!(
        &off.data()[..WIN_W as usize * 44 * 4],
        &on.data()[..WIN_W as usize * 44 * 4]
    );
    assert_ne!(
        &off.data()[WIN_W as usize * 80 * 4..WIN_W as usize * 190 * 4],
        &on.data()[WIN_W as usize * 80 * 4..WIN_W as usize * 190 * 4]
    );
    assert!(off.data().chunks_exact(4).all(|p| p[3] == 255));
}
