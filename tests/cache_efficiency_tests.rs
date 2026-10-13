use snipping_tool_ul::{
    capture::CapturedScreen,
    config::Config,
    geometry::{Color, Point, Size},
    preview::EditorView,
    preview_render::PreviewRenderer,
    state::OverlayState,
    tools::Annotation,
};

fn scene() -> OverlayState {
    let mut pixels = vec![255; 200 * 120 * 4];
    for (i, p) in pixels.chunks_exact_mut(4).enumerate() {
        p[0] = (i % 197) as u8;
        p[1] = (i / 200 % 113) as u8;
    }
    let mut s = OverlayState::new(
        CapturedScreen {
            pixels,
            width: 200,
            height: 120,
            x_offset: 0,
            y_offset: 0,
            scale_factor: 1.,
        },
        &Config::default(),
    );
    s.preview_mode = true;
    s
}

fn matches_fresh(renderer: &mut PreviewRenderer, state: &OverlayState, view: &EditorView) {
    let mut cached = tiny_skia::Pixmap::new(660, 480).unwrap();
    let mut fresh = cached.clone();
    renderer.draw(state, &mut cached, view, 1.);
    PreviewRenderer::default().draw(state, &mut fresh, view, 1.);
    assert_eq!(cached.data(), fresh.data());
}

#[test]
fn whole_pixel_pan_reuses_local_layers_but_fractional_and_clipped_pan_rebuild() {
    let mut state = scene();
    state.annotations.push(Annotation::Arrow {
        start: Point::new(15., 15.),
        end: Point::new(120., 85.),
        color: Color::red(),
        thickness: 5.,
    });
    let mut renderer = PreviewRenderer::default();
    let mut view = EditorView::default();
    matches_fresh(&mut renderer, &state, &view);
    assert!(view.begin_pan(320., 220., 660, 480, 1.));
    for x in 321..341 {
        assert!(view.drag_pan(x as f64, 220.));
        matches_fresh(&mut renderer, &state, &view);
    }
    assert_eq!((renderer.resamples, renderer.compositions), (1, 1));
    assert!(view.drag_pan(340.25, 220.));
    matches_fresh(&mut renderer, &state, &view);
    assert_eq!(renderer.resamples, 2);
    assert!(view.drag_pan(590.25, 220.));
    matches_fresh(&mut renderer, &state, &view);
    assert_eq!(renderer.resamples, 3);
}

#[test]
fn many_small_pixelations_remain_cached_across_zoom_within_budget() {
    let mut state = scene();
    for i in 0..12 {
        state.annotations.push(Annotation::Pixelate {
            top_left: Point::new((i % 6) as f32 * 30., (i / 6) as f32 * 45.),
            size: Size::new(25., 35.),
            block_size: 4,
        });
    }
    let mut renderer = PreviewRenderer::default();
    let mut view = EditorView::default();
    for zoom in [1., 1.125, 1.25, 1.375, 1.5] {
        view.zoom = zoom;
        matches_fresh(&mut renderer, &state, &view);
        assert_eq!(renderer.pixelation_builds(), 12);
        assert!(renderer.pixelation_cache_bytes() <= 1024 * 1024);
    }
}

#[test]
fn fractional_edges_follow_pixel_centers_even_for_tiny_images() {
    let mut state = scene();
    state.screenshot.pixels.fill(255);
    let mut renderer = PreviewRenderer::default();
    for zoom in [1., 0.01, 0.005, 0.003] {
        let mut view = EditorView::default();
        view.zoom = zoom;
        assert!(view.begin_pan(330., 220., 660, 480, 1.));
        for offset in [0., 0.25, 0.5, 0.75] {
            view.drag_pan(330. + offset, 220. + offset);
            let layout = view.framed_layout(200, 120, 660, 480, 1.);
            let well = snipping_tool_ul::editor_ui::EditorUi::canvas_rect(660., 480.);
            let mut frame = tiny_skia::Pixmap::new(660, 480).unwrap();
            renderer.draw(&state, &mut frame, &view, 1.);
            for y in 0..480 {
                for x in 0..660 {
                    let (cx, cy) = (x as f64 + 0.5, y as f64 + 0.5);
                    let inside = cx >= layout.x
                        && cx < layout.x + 200. * layout.scale
                        && cy >= layout.y
                        && cy < layout.y + 120. * layout.scale
                        && cx >= well.x as f64
                        && cx < (well.x + well.w) as f64
                        && cy >= well.y as f64
                        && cy < (well.y + well.h) as f64;
                    assert_eq!(
                        frame.data()[((y * 660 + x) * 4 + 3) as usize] == 255,
                        inside,
                        "zoom={zoom} offset={offset} at {x},{y}"
                    );
                }
            }
        }
    }
}

#[test]
fn transparent_preview_and_pixelation_keep_premultiplied_alpha() {
    let mut state = scene();
    state.screenshot = CapturedScreen {
        pixels: vec![255, 0, 0, 0, 0, 0, 255, 255],
        width: 2,
        height: 1,
        x_offset: 0,
        y_offset: 0,
        scale_factor: 1.,
    };
    state.annotations.push(Annotation::Pixelate {
        top_left: Point::new(0., 0.),
        size: Size::new(2., 1.),
        block_size: 2,
    });
    let mut renderer = PreviewRenderer::default();
    for zoom in [2., 4., 8.] {
        let mut view = EditorView::default();
        view.zoom = zoom;
        let layout = view.framed_layout(2, 1, 660, 480, 1.);
        let mut frame = tiny_skia::Pixmap::new(660, 480).unwrap();
        renderer.draw(&state, &mut frame, &view, 1.);
        let x = (layout.x + layout.scale) as usize;
        let y = (layout.y + layout.scale * 0.5) as usize;
        let i = (y * 660 + x) * 4;
        assert_eq!(&frame.data()[i..i + 4], &[0, 0, 127, 127]);
        assert!(frame
            .data()
            .chunks_exact(4)
            .all(|p| p[..3].iter().all(|c| *c <= p[3])));
    }
    assert_eq!(renderer.pixelation_builds(), 1);
    state.annotations.clear();
    let mut frame = tiny_skia::Pixmap::new(660, 480).unwrap();
    let mut view = EditorView::default();
    view.zoom = 8.;
    renderer.draw(&state, &mut frame, &view, 1.);
    assert!(frame
        .data()
        .chunks_exact(4)
        .all(|p| p[0] == 0 && p[1] == 0 && p[2] <= p[3]));
}
