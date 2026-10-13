use snipping_tool_ul::{
    capture::CapturedScreen,
    config::Config,
    editor_ui::EditorUi,
    geometry::{Point, Size},
    launcher::Action,
    preview::EditorView,
    preview_render::PreviewRenderer,
    state::OverlayState,
    tools::{Annotation, ToolKind},
};

#[test]
fn unchanged_annotation_cache_matches_fresh_render_and_invalidates_edits() {
    let mut s = state(240, 160);
    s.annotations.push(Annotation::Arrow {
        start: Point::new(20., 20.),
        end: Point::new(180., 130.),
        color: snipping_tool_ul::geometry::Color::red(),
        thickness: 6.,
    });
    let mut renderer = PreviewRenderer::default();
    let mut view = EditorView::default();
    let mut frame = tiny_skia::Pixmap::new(660, 390).unwrap();
    renderer.draw(&s, &mut frame, &view, 1.);
    let expected = frame.clone();
    frame.fill(tiny_skia::Color::TRANSPARENT);
    renderer.draw(&s, &mut frame, &view, 1.);
    assert_eq!(renderer.compositions, 1);
    assert_eq!(frame.data(), expected.data());
    for change in 0..6 {
        match change {
            0 => s.selected_index = Some(0),
            1 => snipping_tool_ul::tools::move_annotation(&mut s.annotations[0], 10., 0.),
            2 => {
                s.text_input_active = true;
                s.text_input_buffer = "gyp".into();
                s.text_input_position = Point::new(40., 50.);
            }
            3 => {
                s.text_input_active = false;
                s.annotations.clear();
                s.selected_index = None;
            }
            4 => {
                view.zoom_framed(300., 180., 2., 240, 160, 660, 390, 1.);
            }
            _ => {
                s.active_tool = ToolKind::Eyedropper;
                s.eyedropper_preview = Some(snipping_tool_ul::geometry::Color::blue());
            }
        }
        frame.fill(tiny_skia::Color::TRANSPARENT);
        renderer.draw(&s, &mut frame, &view, 1.);
        let mut fresh = tiny_skia::Pixmap::new(660, 390).unwrap();
        PreviewRenderer::default().draw(&s, &mut fresh, &view, 1.);
        assert_eq!(
            frame.data(),
            fresh.data(),
            "stale pixels after change {change}"
        );
    }
}

#[test]
fn very_large_pencil_marks_do_not_duplicate_unbounded_annotation_metadata() {
    let mut s = state(100, 80);
    s.annotations.push(Annotation::Pencil {
        points: (0..40000)
            .map(|i| Point::new((i % 90) as f32, (i % 70) as f32))
            .collect(),
        color: snipping_tool_ul::geometry::Color::red(),
        thickness: 2.,
    });
    let mut renderer = PreviewRenderer::default();
    let mut p = tiny_skia::Pixmap::new(660, 390).unwrap();
    for _ in 0..2 {
        renderer.draw(&s, &mut p, &EditorView::default(), 1.);
    }
    assert_eq!(
        renderer.compositions, 2,
        "oversized metadata should not be retained"
    );
}
fn state(w: u32, h: u32) -> OverlayState {
    let mut pixels = vec![255; (w * h * 4) as usize];
    for y in 0..h {
        for x in 0..w {
            let i = ((y * w + x) * 4) as usize;
            pixels[i] = (x * 11 % 251) as u8;
            pixels[i + 1] = (y * 7 % 251) as u8;
            pixels[i + 2] = ((x + y) * 3 % 251) as u8;
        }
    }
    let mut s = OverlayState::new(
        CapturedScreen {
            pixels,
            width: w,
            height: h,
            x_offset: 0,
            y_offset: 0,
            scale_factor: 1.,
        },
        &Config::default(),
    );
    s.preview_mode = true;
    s.active_tool = ToolKind::Arrow;
    s
}
#[test]
fn new_capture_waits_for_release_and_fires_once() {
    let s = state(100, 80);
    let mut ui = EditorUi::new(&Config::default());
    let p = Point::new(50., 30.);
    assert_eq!(ui.hit(p, 660., 390., &s), Some(Action::New));
    assert_eq!(ui.release_new_capture(p, 660., 390., &s), None);
    ui.arm_new_capture();
    assert!(ui.new_capture_armed());
    assert_eq!(ui.release_new_capture(p, 660., 390., &s), Some(Action::New));
    assert!(!ui.new_capture_armed());
    assert_eq!(ui.release_new_capture(p, 660., 390., &s), None);
    ui.arm_new_capture();
    assert_eq!(
        ui.release_new_capture(Point::new(320., 220.), 660., 390., &s),
        None
    );
    ui.arm_new_capture();
    ui.cancel_new_capture();
    assert_eq!(ui.release_new_capture(p, 660., 390., &s), None);
}
#[test]
fn space_recenters_only_select_without_typing_or_drags() {
    let mut s = state(100, 80);
    let mut view = EditorView::default();
    view.zoom_framed(300., 180., 4., 100, 80, 660, 390, 1.);
    assert!(!view.recenter_for_select(&s));
    assert!(view.zoom > 1.);
    s.active_tool = ToolKind::Select;
    s.text_input_active = true;
    assert!(!view.recenter_for_select(&s));
    s.text_input_active = false;
    s.select_drag_start = Some(Point::new(3., 4.));
    assert!(!view.recenter_for_select(&s));
    s.select_drag_start = None;
    assert!(view.begin_pan(300., 180., 660, 390, 1.));
    assert!(!view.recenter_for_select(&s));
    view.end_pan();
    assert!(view.recenter_for_select(&s));
    let got = view.framed_layout(100, 80, 660, 390, 1.);
    let expected = EditorView::default().framed_layout(100, 80, 660, 390, 1.);
    assert_eq!(
        (got.x, got.y, got.scale),
        (expected.x, expected.y, expected.scale)
    );
}
#[test]
fn precomputed_sampler_matches_previous_bilinear_pixels() {
    let s = state(317, 233);
    for dpi in [1., 1.5] {
        for zoom in [0.25, 1., 2., 8.] {
            let mut view = EditorView::default();
            view.zoom = zoom;
            let w = (660. * dpi) as u32;
            let h = (390. * dpi) as u32;
            let mut p = tiny_skia::Pixmap::new(w, h).unwrap();
            PreviewRenderer::default().draw(&s, &mut p, &view, dpi);
            let l = view.framed_layout(317, 233, w, h, dpi);
            let well = EditorUi::canvas_rect(w as f32 / dpi as f32, h as f32 / dpi as f32);
            let x0 = l.x.max(well.x as f64 * dpi).max(0.).ceil() as u32;
            let y0 = l.y.max(well.y as f64 * dpi).max(0.).ceil() as u32;
            let x1 = (l.x + 317. * l.scale)
                .min((well.x + well.w) as f64 * dpi)
                .min(w as f64)
                .floor()
                .max(0.) as u32;
            let y1 = (l.y + 233. * l.scale)
                .min((well.y + well.h) as f64 * dpi)
                .min(h as f64)
                .floor()
                .max(0.) as u32;
            for y in y0..y1 {
                for x in x0..x1 {
                    let sx = ((x as f64 + 0.5 - l.x) / l.scale - 0.5).clamp(0., 316.);
                    let sy = ((y as f64 + 0.5 - l.y) / l.scale - 0.5).clamp(0., 232.);
                    let ix = sx.floor() as u32;
                    let iy = sy.floor() as u32;
                    let jx = (ix + 1).min(316);
                    let jy = (iy + 1).min(232);
                    let fx = (sx - ix as f64) as f32;
                    let fy = (sy - iy as f64) as f32;
                    for c in 0..4 {
                        let sample =
                            |xx, yy| s.screenshot.pixels[((yy * 317 + xx) * 4) as usize + c] as f32;
                        let top = sample(ix, iy) * (1. - fx) + sample(jx, iy) * fx;
                        let bottom = sample(ix, jy) * (1. - fx) + sample(jx, jy) * fx;
                        assert_eq!(
                            p.data()[((y * w + x) * 4) as usize + c],
                            (top * (1. - fy) + bottom * fy).round() as u8
                        );
                    }
                }
            }
        }
    }
}
#[test]
fn pixelation_reuses_exact_block_colors_across_zoom_and_invalidates_edits() {
    let mut s = state(120, 80);
    s.annotations.push(Annotation::Pixelate {
        top_left: Point::new(40., 32.),
        size: Size::new(8., 7.),
        block_size: 3,
    });
    let original = s.screenshot.pixels.clone();
    let mut renderer = PreviewRenderer::default();
    let mut p = tiny_skia::Pixmap::new(660, 390).unwrap();
    let mut view = EditorView::default();
    for zoom in [1., 2., 3., 1.] {
        view.zoom = zoom;
        renderer.draw(&s, &mut p, &view, 1.);
        let l = view.framed_layout(120, 80, 660, 390, 1.);
        for by in 0..3 {
            for bx in 0..3 {
                let x = 40 + bx * 3;
                let y = 32 + by * 3;
                let bw = 3.min(48 - x);
                let bh = 3.min(39 - y);
                let mut sum = [0u64; 3];
                for yy in y..y + bh {
                    for xx in x..x + bw {
                        for c in 0..3 {
                            sum[c] += original[((yy * 120 + xx) * 4) as usize + c] as u64;
                        }
                    }
                }
                let px = (l.x + (x as f64 + 0.5) * l.scale).floor() as u32;
                let py = (l.y + (y as f64 + 0.5) * l.scale).floor() as u32;
                for c in 0..3 {
                    assert_eq!(
                        p.data()[((py * 660 + px) * 4) as usize + c],
                        (sum[c] / (bw * bh) as u64) as u8
                    );
                }
            }
        }
        assert_eq!(renderer.pixelation_builds(), 1);
    }
    if let Annotation::Pixelate { block_size, .. } = &mut s.annotations[0] {
        *block_size = 2;
    }
    renderer.draw(&s, &mut p, &view, 1.);
    assert_eq!(renderer.pixelation_builds(), 2);
    assert_eq!(s.screenshot.pixels, original);
}
#[test]
fn tiny_block_fallback_is_bounded_and_samples_source_without_grid_allocation() {
    let mut s = state(700, 700);
    s.annotations.push(Annotation::Pixelate {
        top_left: Point::new(0., 0.),
        size: Size::new(700., 700.),
        block_size: 1,
    });
    let view = EditorView::default();
    let mut p = tiny_skia::Pixmap::new(660, 390).unwrap();
    let mut renderer = PreviewRenderer::default();
    renderer.draw(&s, &mut p, &view, 1.);
    assert_eq!(renderer.pixelation_cache_bytes(), 0);
    assert_eq!(renderer.pixelation_builds(), 0);
    let l = view.framed_layout(700, 700, 660, 390, 1.);
    let x = 330u32;
    let y = 180u32;
    let sx = ((x as f64 + 0.5 - l.x) / l.scale).floor() as u32;
    let sy = ((y as f64 + 0.5 - l.y) / l.scale).floor() as u32;
    assert_eq!(
        &p.data()[((y * 660 + x) * 4) as usize..((y * 660 + x) * 4) as usize + 3],
        &s.screenshot.pixels[((sy * 700 + sx) * 4) as usize..((sy * 700 + sx) * 4) as usize + 3]
    );
}
#[test]
fn pixelation_cache_evicts_under_one_mib_budget() {
    let mut s = state(500, 500);
    let mut renderer = PreviewRenderer::default();
    let mut p = tiny_skia::Pixmap::new(660, 390).unwrap();
    for offset in 0..4 {
        s.annotations = vec![Annotation::Pixelate {
            top_left: Point::new(offset as f32, offset as f32),
            size: Size::new(420., 420.),
            block_size: 1,
        }];
        renderer.draw(&s, &mut p, &EditorView::default(), 1.);
        assert!(renderer.pixelation_cache_bytes() <= 1024 * 1024);
    }
    assert_eq!(renderer.pixelation_builds(), 4);
}
