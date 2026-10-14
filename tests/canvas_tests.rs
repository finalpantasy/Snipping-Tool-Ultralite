use snipping_tool_ul::{
    canvas::{self, CanvasBounds},
    capture::CapturedScreen,
    config::Config,
    editor_ui::EditorUi,
    export,
    geometry::{Color, Point, Size},
    overlay::selection::Selection,
    preview::EditorView,
    preview_render::PreviewRenderer,
    state::OverlayState,
    tools::{self, Annotation, AnnotationTool, ResizeHandle, ToolKind, UndoAction},
};

fn state() -> OverlayState {
    let mut o = OverlayState::new(
        CapturedScreen {
            pixels: [20, 100, 150, 255].repeat(100 * 80),
            width: 100,
            height: 80,
            x_offset: 0,
            y_offset: 0,
            scale_factor: 1.,
        },
        &Config::default(),
    );
    o.preview_mode = true;
    o.selection = Some(Selection {
        x: 0.,
        y: 0.,
        width: 100.,
        height: 80.,
    });
    o
}
fn rectangle(x: f32, y: f32, w: f32, h: f32) -> Annotation {
    Annotation::Rectangle {
        top_left: Point::new(x, y),
        size: Size::new(w, h),
        color: Color::red(),
        thickness: 4.,
    }
}
fn frame(
    o: &OverlayState,
    renderer: &mut PreviewRenderer,
    view: &EditorView,
    dpi: f64,
) -> tiny_skia::Pixmap {
    let mut p = tiny_skia::Pixmap::new((900. * dpi) as u32, (650. * dpi) as u32).unwrap();
    EditorUi::new(&Config::default()).draw_base(&mut p, dpi as f32);
    renderer.draw(o, &mut p, view, dpi);
    p
}
fn at(p: &tiny_skia::Pixmap, view: &EditorView, dpi: f64, x: f64, y: f64) -> [u8; 4] {
    let l = view.framed_layout(100, 80, p.width(), p.height(), dpi);
    let px = (l.x + x * l.scale).floor() as usize;
    let py = (l.y + y * l.scale).floor() as usize;
    p.data()[(py * p.width() as usize + px) * 4..][..4]
        .try_into()
        .unwrap()
}

#[test]
fn canvas_includes_all_four_stroked_edges_and_keeps_original_pixels() {
    let mut o = state();
    o.annotations.push(rectangle(-20., -15., 150., 115.));
    assert_eq!(
        o.editor_canvas_bounds(),
        CanvasBounds {
            x: -22.,
            y: -17.,
            width: 154.,
            height: 119.
        }
    );
    let (pixels, w, h) = o.flattened_selection().unwrap();
    assert_eq!((w, h), (154, 119));
    assert_eq!(pixels.len(), w as usize * h as usize * 4);
    // All four complete edges are present, including their outer stroke halves.
    for (x, y) in [(0, 60), (153, 60), (60, 0), (60, 118)] {
        assert_eq!(
            &pixels[((y * w + x) * 4) as usize..][..4],
            &[255, 0, 0, 255]
        );
    }
    assert_eq!(
        &pixels[((40 * w + 60) * 4) as usize..][..4],
        &[20, 100, 150, 255]
    );
    assert_eq!(&pixels[((10 * w + 10) * 4) as usize..][..4], &[255; 4]);
    assert_eq!(o.screenshot.width, 100);
    assert_eq!(o.screenshot.pixels, [20, 100, 150, 255].repeat(100 * 80));
    let (raw, rw, rh, annotations) = o.export_selection().unwrap();
    assert_eq!((rw, rh), (w, h));
    assert_eq!(
        export::flatten_annotations(&raw, rw, rh, &annotations),
        pixels
    );
}

#[test]
fn resize_move_delete_undo_and_redo_derive_fresh_extents() {
    let mut o = state();
    o.annotations.push(rectangle(30., 30., 100., 70.));
    let large = o.editor_canvas_bounds();
    assert_eq!((large.width, large.height), (132., 102.));
    let old = o.annotations[0].clone();
    tools::resize_annotation(
        &mut o.annotations[0],
        ResizeHandle::BottomRight,
        Point::new(70., 60.),
    );
    tools::record_undo(
        &mut o.undo_stack,
        &mut o.redo_stack,
        UndoAction::Modify(0, old),
    );
    assert_eq!(o.editor_canvas_bounds(), CanvasBounds::image(100, 80));
    assert!(tools::apply_undo(
        &mut o.annotations,
        &mut o.undo_stack,
        &mut o.redo_stack
    ));
    assert_eq!(o.editor_canvas_bounds(), large);
    assert!(tools::apply_redo(
        &mut o.annotations,
        &mut o.undo_stack,
        &mut o.redo_stack
    ));
    assert_eq!(o.editor_canvas_bounds(), CanvasBounds::image(100, 80));
    tools::move_annotation(&mut o.annotations[0], -100., -100.);
    assert!(o.editor_canvas_bounds().x < 0.);
    let moved = o.editor_canvas_bounds();
    assert!(o.delete_annotation(0));
    assert_eq!(o.editor_canvas_bounds(), CanvasBounds::image(100, 80));
    assert_eq!(o.flattened_selection().unwrap().1, 100);
    assert!(tools::apply_undo(
        &mut o.annotations,
        &mut o.undo_stack,
        &mut o.redo_stack
    ));
    assert_eq!(o.editor_canvas_bounds(), moved);
    assert!(tools::apply_redo(
        &mut o.annotations,
        &mut o.undo_stack,
        &mut o.redo_stack
    ));
    assert_eq!(o.editor_canvas_bounds(), CanvasBounds::image(100, 80));
}

#[test]
fn remaining_marks_control_shrink_and_stroke_size_edits() {
    let mut o = state();
    o.annotations = vec![
        rectangle(30., 30., 100., 70.),
        rectangle(-50., -30., 60., 40.),
    ];
    o.delete_annotation(0);
    let b = o.editor_canvas_bounds();
    assert_eq!((b.x, b.y, b.width, b.height), (-52., -32., 152., 112.));
    o.selected_index = Some(0);
    o.set_tool_size(20., true);
    assert_eq!(o.editor_canvas_bounds().x, -60.);
    o.set_tool_size(2., true);
    assert_eq!(o.editor_canvas_bounds().x, -51.);
}

#[test]
fn preview_displays_growth_and_shrink_without_moving_the_pointer_transform() {
    let mut o = state();
    let mut renderer = PreviewRenderer::default();
    let view = EditorView::default();
    let original = frame(&o, &mut renderer, &view, 1.);
    let point = view.framed_point(500., 350., 100, 80, 900, 650, 1.);
    o.active_tool = ToolKind::Rectangle;
    o.rectangle_tool.on_mouse_down(Point::new(50., 40.));
    o.rectangle_tool.on_mouse_move(Point::new(145., 110.));
    let live = frame(&o, &mut renderer, &view, 1.);
    assert_eq!(at(&live, &view, 1., 120., 90.), [255; 4]);
    let border = at(&live, &view, 1., 144., 90.);
    assert!(border[0] > 240 && border[1] < 30);
    assert_eq!(point, view.framed_point(500., 350., 100, 80, 900, 650, 1.));
    let ann = o
        .rectangle_tool
        .on_mouse_up(Point::new(145., 110.))
        .unwrap();
    o.annotations.push(ann);
    let committed = frame(&o, &mut renderer, &view, 1.);
    assert_eq!(live.data(), committed.data());
    o.delete_annotation(0);
    let shrunk = frame(&o, &mut renderer, &view, 1.);
    assert_eq!(original.data(), shrunk.data());
}

#[test]
fn live_text_growth_and_cancellation_leave_no_margin() {
    let mut o = state();
    o.text_input_active = true;
    o.text_input_position = Point::new(90., -25.);
    o.text_input_font_size = 32.;
    o.text_input_buffer = "Outside gyp".into();
    let b = o.editor_canvas_bounds();
    assert!(b.y < 0. && b.width > 100.);
    let (x, y, w, h) = tools::annotation_bounding_box(&Annotation::Text {
        position: o.text_input_position,
        text: o.text_input_buffer.clone(),
        color: Color::red(),
        font_size: 32.,
    })
    .unwrap();
    assert!(b.x <= x && b.y <= y && b.x + b.width >= x + w && b.y + b.height >= y + h);
    o.text_input_buffer.clear();
    assert_eq!(o.editor_canvas_bounds(), CanvasBounds::image(100, 80));
}

#[test]
fn image_effects_do_not_expand_or_pixelate_white_padding() {
    let mut o = state();
    o.annotations.push(Annotation::Pixelate {
        top_left: Point::new(-20., -20.),
        size: Size::new(160., 120.),
        block_size: 13,
    });
    assert_eq!(o.editor_canvas_bounds(), CanvasBounds::image(100, 80));
    let source_only = o.flattened_selection().unwrap().0;
    o.annotations.push(rectangle(-25., -25., 160., 120.));
    let (pixels, w, _) = o.flattened_selection().unwrap();
    // Clipping before offset keeps the pixelation grid identical inside the source.
    for row in 0..80usize {
        let dst = ((row + 27) * w as usize + 27) * 4;
        assert_eq!(
            &pixels[dst..dst + 400],
            &source_only[row * 400..(row + 1) * 400]
        );
    }
    assert_eq!(&pixels[((10 * w + 10) * 4) as usize..][..4], &[255; 4]);
    let b = canvas::bounds(
        100,
        80,
        &[Annotation::Spotlight {
            top_left: Point::new(-500., -500.),
            size: Size::new(1000., 1000.),
        }],
    );
    assert_eq!(b, CanvasBounds::image(100, 80));
}

#[test]
fn imported_alpha_is_preserved_in_source_and_new_margin_is_white() {
    let pixels = vec![160, 80, 40, 128];
    let ann = Annotation::Highlight {
        top_left: Point::new(-3., -3.),
        size: Size::new(1., 1.),
        color: Color::red(),
    };
    let (raw, w, h, _) = canvas::prepare_export(pixels.clone(), 1, 1, vec![ann]).unwrap();
    assert_eq!((w, h), (4, 4));
    assert_eq!(&raw[(3 * w + 3) as usize * 4..], &pixels);
    assert_eq!(&raw[4..8], &[255; 4]);
}

#[test]
fn pan_zoom_and_dpi_can_view_marks_when_original_image_is_offscreen() {
    let mut o = state();
    o.annotations.push(rectangle(300., 20., 60., 60.));
    for dpi in [1., 1.5, 2.] {
        let mut view = EditorView::default();
        view.zoom = 2.;
        view.begin_pan(
            450. * dpi,
            350. * dpi,
            (900. * dpi) as u32,
            (650. * dpi) as u32,
            dpi,
        );
        view.drag_pan(150. * dpi, 350. * dpi);
        view.end_pan();
        let mut renderer = PreviewRenderer::default();
        let p = frame(&o, &mut renderer, &view, dpi);
        let color = at(&p, &view, dpi, 300., 45.);
        assert!(
            color[0] > 240 && color[1] < 30,
            "dpi={dpi}, pixel={color:?}"
        );
        let repeats = frame(&o, &mut renderer, &view, dpi);
        assert_eq!(repeats.data(), p.data());
        assert_eq!(renderer.resamples, 1);
        assert_eq!(renderer.compositions, 1);
    }
}

#[test]
fn extreme_extent_does_not_allocate_world_sized_preview_or_export() {
    let mut o = state();
    o.annotations
        .push(rectangle(10., 10., 1_000_000., 1_000_000.));
    assert!(o.flattened_selection().is_none());
    let mut renderer = PreviewRenderer::default();
    let _ = frame(&o, &mut renderer, &EditorView::default(), 1.);
    assert!(renderer.retained_bytes() < 900 * 650 * 4 * 3 + 1024);
    assert!(canvas::prepare_export(vec![0; 3], 1, 1, vec![]).is_none());
    assert!(CanvasBounds::image(65_536, 65_536).export_size().is_none());
}

#[test]
fn editor_padding_allows_new_marks_and_capture_selection_stays_bounded() {
    let o = state();
    assert!(o.selection_hit(Point::new(-40., -30.), true).is_some());
    assert!(o.selection_hit(Point::new(140., 110.), true).is_some());
    assert!(o.selection_hit(Point::new(f32::NAN, 0.), true).is_none());
    assert!(o.selection_hit(Point::new(-40., -30.), false).is_none());
}

#[test]
fn unedited_image_and_capture_overlay_do_not_gain_a_margin() {
    let mut o = state();
    let (pixels, w, h) = o.flattened_selection().unwrap();
    assert_eq!((w, h), (100, 80));
    assert_eq!(pixels, o.screenshot.pixels);
    o.preview_mode = false;
    o.annotations.push(rectangle(-20., -10., 150., 120.));
    let (_, w, h) = o.flattened_selection().unwrap();
    assert_eq!((w, h), (100, 80));
}

#[test]
fn derived_bounds_contain_every_tool_pixel_in_an_unclipped_reference() {
    let marks = vec![
        rectangle(-10., -10., 140., 110.),
        Annotation::RoundedRect {
            top_left: Point::new(-10., -10.),
            size: Size::new(140., 110.),
            color: Color::red(),
            thickness: 20.,
            radius: 12.,
        },
        Annotation::Ellipse {
            center: Point::new(50., 40.),
            radius_x: 70.,
            radius_y: 60.,
            color: Color::red(),
            thickness: 20.,
        },
        Annotation::Line {
            start: Point::new(-10., -10.),
            end: Point::new(140., 110.),
            color: Color::red(),
            thickness: 20.,
        },
        Annotation::Arrow {
            start: Point::new(30., 30.),
            end: Point::new(-20., -20.),
            color: Color::red(),
            thickness: 20.,
        },
        Annotation::Pencil {
            points: vec![
                Point::new(-10., -10.),
                Point::new(20., -20.),
                Point::new(140., 110.),
            ],
            color: Color::red(),
            thickness: 20.,
        },
        Annotation::Highlight {
            top_left: Point::new(-10., -10.),
            size: Size::new(140., 110.),
            color: Color::red(),
        },
        Annotation::Text {
            position: Point::new(-10., -10.),
            text: "gyp jq".into(),
            color: Color::red(),
            font_size: 32.,
        },
        Annotation::StepMarker {
            position: Point::new(-10., -10.),
            number: 99,
            color: Color::red(),
            size: 60.,
        },
        Annotation::Measurement {
            start: Point::new(-10., -10.),
            end: Point::new(-10., 110.),
            color: Color::red(),
        },
    ];
    for a in marks {
        let b = canvas::bounds(100, 80, std::slice::from_ref(&a));
        let mut p = tiny_skia::Pixmap::new(500, 400).unwrap();
        tools::render_annotation(
            &export::offset_annotation(&a, -200., -150.),
            &mut p,
            None,
            None,
        );
        let mut count = 0;
        for (i, pixel) in p.data().chunks_exact(4).enumerate() {
            if pixel[3] == 0 {
                continue;
            }
            count += 1;
            let x = (i % 500) as f32 - 200.;
            let y = (i / 500) as f32 - 150.;
            assert!(
                x >= b.x && y >= b.y && x < b.x + b.width && y < b.y + b.height,
                "Clipped {a:?} at {x},{y}, bounds={b:?}"
            );
        }
        assert!(count > 0);
    }
}

#[test]
fn fractional_pixelation_at_image_edges_survives_canvas_expansion_exactly() {
    for (x, y, w, h) in [
        (95.9, 73.9, 20.5, 20.5),
        (-2.4, -3.2, 8.7, 9.9),
        (0.999999, 0.999999, 6.5, 7.5),
    ] {
        let mut o = state();
        for (i, p) in o.screenshot.pixels.chunks_exact_mut(4).enumerate() {
            p[0] = (i % 251) as u8;
            p[1] = (i / 100 * 3) as u8;
            p[2] = (i % 100 * 2) as u8;
        }
        o.annotations.push(Annotation::Pixelate {
            top_left: Point::new(x, y),
            size: Size::new(w, h),
            block_size: 13,
        });
        let original = o.flattened_selection().unwrap().0;
        o.annotations.push(rectangle(-40., -30., 180., 140.));
        let b = o.editor_canvas_bounds();
        let (expanded, ew, _) = o.flattened_selection().unwrap();
        for row in 0..80usize {
            let dst = ((row + (-b.y) as usize) * ew as usize + (-b.x) as usize) * 4;
            assert_eq!(
                &expanded[dst..dst + 400],
                &original[row * 400..(row + 1) * 400],
                "region=({x},{y},{w},{h}), row={row}"
            );
        }
    }
}

#[test]
fn empty_source_cannot_be_exported_as_an_annotation_only_canvas() {
    let mut o = state();
    o.screenshot.width = 0;
    o.screenshot.pixels.clear();
    o.annotations.push(rectangle(-10., -10., 150., 120.));
    assert!(o.export_selection().is_none());
}

#[test]
fn empty_source_preview_is_safe_even_with_annotations() {
    let mut o = state();
    o.screenshot.width = 0;
    o.screenshot.pixels.clear();
    o.annotations.push(rectangle(-10., -10., 150., 120.));
    let mut renderer = PreviewRenderer::default();
    let _ = frame(&o, &mut renderer, &EditorView::default(), 1.);
}

#[test]
fn fractional_spotlight_source_pixels_survive_canvas_expansion_exactly() {
    let mut o = state();
    o.annotations.push(Annotation::Spotlight {
        top_left: Point::new(-2.4, -3.2),
        size: Size::new(8.7, 9.9),
    });
    let original = o.flattened_selection().unwrap().0;
    o.annotations.push(rectangle(-40., -30., 180., 140.));
    let b = o.editor_canvas_bounds();
    let (expanded, w, _) = o.flattened_selection().unwrap();
    for row in 0..80usize {
        let dst = ((row + (-b.y) as usize) * w as usize + (-b.x) as usize) * 4;
        assert!(
            &expanded[dst..dst + 400] == &original[row * 400..(row + 1) * 400],
            "Spotlight changed at row={row}"
        );
    }
}

#[test]
fn effect_pixel_stability_stress_includes_alpha_fractional_edges_and_outside_masks() {
    let cases = [
        (-200., -150., 10., 10.),
        (-2.4, -3.2, 8.7, 9.9),
        (95.9, 73.9, 20.5, 20.5),
        (0.999999, 0.999999, 6.5, 7.5),
        (18.1, 24.8, 110.4, 80.7),
        (25.5, 20.1, 30.2, 25.8),
    ];
    for alpha in [0, 64, 128, 255] {
        for &(x, y, w, h) in &cases {
            for effect in 0..3 {
                let mut o = state();
                for (i, p) in o.screenshot.pixels.chunks_exact_mut(4).enumerate() {
                    p.copy_from_slice(&[
                        (i % 251) as u8,
                        (i / 100 * 3) as u8,
                        (i % 100 * 2) as u8,
                        alpha,
                    ]);
                }
                if effect != 1 {
                    o.annotations.push(Annotation::Pixelate {
                        top_left: Point::new(x, y),
                        size: Size::new(w, h),
                        block_size: 13,
                    });
                }
                if effect != 0 {
                    o.annotations.push(Annotation::Spotlight {
                        top_left: Point::new(x, y),
                        size: Size::new(w, h),
                    });
                }
                let original = o.flattened_selection().unwrap().0;
                o.annotations.push(rectangle(-300., -240., 710., 570.));
                let b = o.editor_canvas_bounds();
                let (expanded, ew, _) = o.flattened_selection().unwrap();
                for row in 0..80usize {
                    let dst = ((row + (-b.y) as usize) * ew as usize + (-b.x) as usize) * 4;
                    assert!(
                        &expanded[dst..dst + 400] == &original[row * 400..(row + 1) * 400],
                        "alpha={alpha}, effect={effect}, region=({x},{y},{w},{h}), row={row}"
                    );
                }
            }
        }
    }
}

#[test]
fn cached_growth_shrink_and_offscreen_frames_match_fresh_composition() {
    let mut o = state();
    let mut renderer = PreviewRenderer::default();
    let mut seed = 1769u32;
    for n in 0..180 {
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        let x = (seed % 701) as f32 - 350.25;
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        let y = (seed % 501) as f32 - 250.75;
        o.annotations = if n % 7 == 0 {
            vec![]
        } else {
            vec![rectangle(x, y, (n % 120 + 2) as f32, (n % 80 + 2) as f32)]
        };
        if n % 3 == 0 {
            o.annotations.push(Annotation::Spotlight {
                top_left: Point::new(-2.4, 8.9),
                size: Size::new(68.7, 52.2),
            });
        }
        o.selected_index = if n % 5 == 0 && !o.annotations.is_empty() {
            Some(0)
        } else {
            None
        };
        let mut view = EditorView::default();
        view.zoom = [0.1, 0.7, 1., 2., 16.][n % 5];
        view.begin_pan(450., 350., 900, 650, 1.);
        view.drag_pan(450. + (n % 6) as f64 * 210., 350. - (n % 4) as f64 * 180.);
        view.end_pan();
        let cached = frame(&o, &mut renderer, &view, 1.);
        let fresh = frame(&o, &mut PreviewRenderer::default(), &view, 1.);
        assert!(
            cached.data() == fresh.data(),
            "Cached canvas differs at transition={n}"
        );
        assert!(renderer.retained_bytes() < 900 * 650 * 4 * 3 + 1024);
        let again = frame(&o, &mut renderer, &view, 1.);
        assert!(again.data() == cached.data());
    }
}

#[test]
fn pin_origin_includes_expansion_and_preserves_negative_monitor_coordinates() {
    let mut o = state();
    o.screenshot.x_offset = -1920;
    o.screenshot.y_offset = -200;
    o.annotations.push(rectangle(-40., -30., 180., 140.));
    assert_eq!(o.export_screen_origin(), Some((-1962, -232)));
    let b = o.export_canvas_bounds().unwrap();
    let (_, w, h) = o.flattened_selection().unwrap();
    assert_eq!((b.width as u32, b.height as u32), (w, h));
    o.preview_mode = false;
    assert_eq!(o.export_screen_origin(), Some((-1920, -200)));
    o.selection = Some(Selection {
        x: 10.4,
        y: 12.7,
        width: 60.2,
        height: 40.5,
    });
    o.preview_mode = true;
    let b = o.export_canvas_bounds().unwrap();
    let (_, w, h) = o.flattened_selection().unwrap();
    assert_eq!((b.width as u32, b.height as u32), (w, h));
    assert_eq!(o.export_screen_origin(), Some((-1962, -232)));
}

#[test]
fn rejected_canvas_exports_have_an_actionable_failure_message_and_keep_state() {
    let mut o = state();
    o.annotations
        .push(rectangle(10., 10., 1_000_000., 1_000_000.));
    let image = o.screenshot.pixels.clone();
    let marks = o.annotations.clone();
    assert!(o.flattened_selection().is_none());
    assert!(o.export_failure_message().contains("Canvas is too large"));
    assert!(o.export_failure_message().contains("Move or resize"));
    assert_eq!(o.screenshot.pixels, image);
    assert_eq!(o.annotations, marks);
    o.annotations.clear();
    assert!(o.export_failure_message().contains("try again"));
    assert!(canvas::source_region(Point::new(f32::NAN, 0.), Size::new(5., 5.), 100, 80).is_none());
}
