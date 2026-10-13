//! Authored pixels only: no desktop capture, windows or clipboard access.
use snipping_tool_ul::capture::{CapturedScreen, ScreenRect};

fn snapshot() -> CapturedScreen {
    CapturedScreen {
        pixels: (0..12).flat_map(|i| [i, 200, 100, 255]).collect(),
        width: 4,
        height: 3,
        x_offset: -3,
        y_offset: -2,
        scale_factor: 1.5,
    }
}

#[test]
fn crop_uses_frozen_pixels_even_when_live_content_changes() {
    let mut live = snapshot();
    let frozen = live.clone();
    live.pixels.fill(0); // A video advances or a new popup covers the live screen.
    let snip = frozen
        .into_region(ScreenRect {
            x: -2,
            y: -1,
            width: 2,
            height: 2,
        })
        .unwrap();
    assert_eq!(
        snip.pixels,
        [5, 6, 9, 10]
            .into_iter()
            .flat_map(|i| [i, 200, 100, 255])
            .collect::<Vec<_>>()
    );
    assert_eq!(
        (snip.x_offset, snip.y_offset, snip.width, snip.height),
        (-2, -1, 2, 2)
    );
    assert_eq!(snip.scale_factor, 1.5);
}

#[test]
fn full_screen_reuses_buffer_without_allocation() {
    let frozen = snapshot();
    let pointer = frozen.pixels.as_ptr();
    let bounds = frozen.bounds();
    let snip = frozen.into_region(bounds).unwrap();
    assert_eq!(snip.pixels.as_ptr(), pointer);
    assert_eq!(snip.bounds(), bounds);
}

#[test]
fn crop_clips_all_edges_at_negative_monitor_origin() {
    let snip = snapshot()
        .into_region(ScreenRect {
            x: -5,
            y: -4,
            width: 4,
            height: 4,
        })
        .unwrap();
    assert_eq!(
        snip.bounds(),
        ScreenRect {
            x: -3,
            y: -2,
            width: 2,
            height: 2
        }
    );
    assert_eq!(
        snip.pixels,
        [0, 1, 4, 5]
            .into_iter()
            .flat_map(|i| [i, 200, 100, 255])
            .collect::<Vec<_>>()
    );
}

#[test]
fn one_pixel_selection_is_exact_and_does_not_retain_desktop_capacity() {
    let snip = snapshot()
        .into_region(ScreenRect {
            x: 0,
            y: 0,
            width: 1,
            height: 1,
        })
        .unwrap();
    assert_eq!(snip.pixels, [11, 200, 100, 255]);
    assert!(snip.pixels.capacity() < 48);
}

#[test]
fn rejects_empty_offscreen_and_malformed_snapshots() {
    for rect in [
        ScreenRect {
            x: 0,
            y: 0,
            width: 0,
            height: 1,
        },
        ScreenRect {
            x: 100,
            y: 0,
            width: 2,
            height: 2,
        },
    ] {
        assert!(snapshot().into_region(rect).is_err());
    }
    for size in [0, 47, 49] {
        let mut frozen = snapshot();
        frozen.pixels.resize(size, 255);
        let bounds = frozen.bounds();
        assert!(frozen.into_region(bounds).is_err());
    }
}

#[test]
fn rejects_overflow_dimensions_before_reading_or_allocating() {
    let mut frozen = snapshot();
    frozen.width = u32::MAX;
    frozen.height = u32::MAX;
    let bounds = frozen.bounds();
    assert!(frozen.into_region(bounds).is_err());
}
