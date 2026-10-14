use snipping_tool_ul::{
    capture::CapturedScreen,
    config::Config,
    editing::SizeKind,
    editor_ui::EditorUi,
    geometry::{Color, Point, Size},
    overlay::selection::Selection,
    preview::EditorView,
    preview_render::PreviewRenderer,
    state::OverlayState,
    tools::{self, Annotation, AnnotationTool, ToolKind, UndoAction},
};
fn state(w: u32, h: u32) -> OverlayState {
    let mut o = OverlayState::new(
        CapturedScreen {
            pixels: vec![255; (w * h * 4) as usize],
            width: w,
            height: h,
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
        width: w as f32,
        height: h as f32,
    });
    o
}
#[test]
fn live_sizes_and_selected_annotation_edits_are_reversible() {
    let mut o = state(200, 100);
    o.active_tool = ToolKind::Text;
    o.set_tool_size(36., true);
    assert_eq!(o.tool_size(), 36.);
    o.text_input_active = true;
    o.text_input_buffer = "gyp jq".into();
    o.commit_text();
    o.selected_index = Some(0);
    o.set_tool_size(48., true);
    assert!(matches!(
        o.annotations[0],
        Annotation::Text { font_size: 48., .. }
    ));
    assert!(tools::apply_undo(
        &mut o.annotations,
        &mut o.undo_stack,
        &mut o.redo_stack
    ));
    assert!(matches!(
        o.annotations[0],
        Annotation::Text { font_size: 36., .. }
    ));
}
#[test]
fn deleting_middle_step_renumbers_atomically_and_survives_more_than_fifty_steps() {
    let mut o = state(200, 100);
    for n in 1..=100 {
        o.annotations.push(Annotation::StepMarker {
            position: Point::new(n as f32, 50.),
            number: n,
            color: Color::red(),
            size: 28.,
        });
    }
    let before = o.annotations.clone();
    assert!(o.delete_annotation(3));
    assert!(matches!(
        o.annotations[3],
        Annotation::StepMarker { number: 4, .. }
    ));
    assert_eq!(o.undo_stack.len(), 1);
    assert!(tools::apply_undo(
        &mut o.annotations,
        &mut o.undo_stack,
        &mut o.redo_stack
    ));
    assert_eq!(o.annotations, before);
    assert!(tools::apply_redo(
        &mut o.annotations,
        &mut o.undo_stack,
        &mut o.redo_stack
    ));
    assert_eq!(o.annotations.len(), 99);
    assert!(matches!(
        o.annotations[98],
        Annotation::StepMarker { number: 99, .. }
    ));
}
#[test]
fn stale_group_cannot_partly_modify_the_document() {
    let mut o = state(200, 100);
    o.annotations.push(Annotation::StepMarker {
        position: Point::new(20., 20.),
        number: 1,
        color: Color::red(),
        size: 28.,
    });
    let before = o.annotations.clone();
    let mut stack = vec![UndoAction::Batch(vec![
        UndoAction::Delete(0, before[0].clone()),
        UndoAction::Modify(999, before[0].clone()),
    ])];
    assert!(!tools::apply_undo(
        &mut o.annotations,
        &mut stack,
        &mut o.redo_stack
    ));
    assert_eq!(o.annotations, before);
}
#[test]
fn slider_drag_records_one_undo_and_context_controls_fit_minimum_editor() {
    let mut o = state(200, 100);
    o.annotations.push(Annotation::Arrow {
        start: Point::new(10., 10.),
        end: Point::new(100., 50.),
        color: Color::red(),
        thickness: 3.,
    });
    o.selected_index = Some(0);
    let mut ui = EditorUi::new(&Config::default());
    ui.begin_size_drag(&o);
    for n in 4..15 {
        o.set_tool_size(n as f32, false);
    }
    ui.end_size_drag(&mut o);
    assert_eq!(o.undo_stack.len(), 1);
    assert_eq!(o.size_kind(), SizeKind::Width);
    assert!(tools::apply_undo(
        &mut o.annotations,
        &mut o.undo_stack,
        &mut o.redo_stack
    ));
    assert_eq!(o.tool_size(), 3.);
    let r = EditorUi::size_slider(660., 390., &o);
    assert!(r.x >= 0. && r.x + r.w <= 660. && r.y + r.h < 300.);
}
#[test]
fn preview_cache_does_not_resample_for_moves_or_retain_full_resolution_render_targets() {
    let mut o = state(3840, 2160);
    o.annotations.push(Annotation::Arrow {
        start: Point::new(50., 50.),
        end: Point::new(150., 120.),
        color: Color::red(),
        thickness: 3.,
    });
    let mut frame = tiny_skia::Pixmap::new(940, 612).unwrap();
    let mut renderer = PreviewRenderer::default();
    let view = EditorView::default();
    for _ in 0..5 {
        renderer.draw(&o, &mut frame, &view, 1.);
        tools::move_annotation(&mut o.annotations[0], 5., 2.);
    }
    assert_eq!(renderer.resamples, 1);
    assert!(renderer.retained_bytes() < frame.data().len() * 2);
    assert!(o.screenshot.pixels.iter().all(|v| *v == 255));
}
#[test]
fn preview_region_guides_never_enter_exported_pixels() {
    let mut o = state(200, 100);
    o.annotations.push(Annotation::Pixelate {
        top_left: Point::new(20., 20.),
        size: Size::new(60., 30.),
        block_size: 10,
    });
    o.selected_index = Some(0);
    let mut p = tiny_skia::Pixmap::new(940, 612).unwrap();
    let mut renderer = PreviewRenderer::default();
    renderer.draw(&o, &mut p, &EditorView::default(), 1.);
    assert!(p.data().chunks_exact(4).any(|v| v[2] > v[0] && v[2] > v[1]));
    assert!(o.flattened_selection().unwrap().0.iter().all(|v| *v == 255));
}
#[test]
fn text_bounds_include_descenders_and_negative_bearings() {
    {
        let ann = Annotation::Text {
            position: Point::new(20., 20.),
            text: "jgy pq É".into(),
            color: Color::red(),
            font_size: 37.,
        };
        let (x, y, w, h) = tools::annotation_bounding_box(&ann).unwrap();
        let mut p = tiny_skia::Pixmap::new(320, 100).unwrap();
        tools::render_annotation(&ann, &mut p, None, None);
        for yy in 0..100 {
            for xx in 0..320 {
                if p.pixel(xx, yy).unwrap().alpha() > 0 {
                    assert!(
                        xx as f32 >= x
                            && (xx as f32) < x + w
                            && yy as f32 >= y
                            && (yy as f32) < y + h
                    );
                }
            }
        }
    }
}
#[test]
fn pencil_preserves_close_cursor_samples_and_renders_a_single_click() {
    let mut o = state(200, 100);
    o.pencil_tool.on_mouse_down(Point::new(30., 30.));
    for i in 1..10 {
        o.pencil_tool
            .on_mouse_move(Point::new(30. + i as f32 * 0.6, 30.));
    }
    let a = o.pencil_tool.on_mouse_up(Point::new(36., 30.)).unwrap();
    assert!(matches!(a,Annotation::Pencil{ref points,..} if points.len()>=10));
    o.pencil_tool.on_mouse_down(Point::new(40., 40.));
    let dot = o.pencil_tool.on_mouse_up(Point::new(40., 40.)).unwrap();
    let mut p = tiny_skia::Pixmap::new(100, 100).unwrap();
    tools::render_annotation(&dot, &mut p, None, None);
    assert!(p.pixel(40, 40).unwrap().alpha() > 0);
}

#[test]
fn annotation_menu_survives_a_minimized_zero_size_editor() {
    // A minimized window reports 0x0; clamp bounds must not invert and panic.
    let o = state(200, 100);
    let mut ui = EditorUi::new(&Config::default());
    ui.header.menu = snipping_tool_ul::launcher::Menu::Annotation;
    let _ = ui.hover(Point::new(-1., -1.), 0., 0., &o);
    let _ = ui.hit(Point::new(5., 5.), 0., 0., &o);
}
#[test]
fn eyedropper_has_no_color_outside_the_image() {
    let mut o = state(4, 3);
    o.screenshot.pixels[..4].copy_from_slice(&[255, 0, 0, 255]);
    let red = Color::new(1., 0., 0., 1.);
    assert_eq!(o.sample_color(&Point::new(0.5, 0.5)), Some(red));
    // Left of or above the image used to clamp onto the first column or row.
    assert_eq!(o.sample_color(&Point::new(-3., 0.5)), None);
    assert_eq!(o.sample_color(&Point::new(0.5, -0.2)), None);
    assert_eq!(o.sample_color(&Point::new(4., 1.)), None);
    assert_eq!(o.sample_color(&Point::new(1., 3.)), None);
    assert_eq!(o.sample_color(&Point::new(f32::NAN, 1.)), None);
}
#[test]
fn switching_tools_mid_drag_leaves_no_stale_shape() {
    let mut o = state(200, 100);
    o.active_tool = ToolKind::Rectangle;
    o.rectangle_tool.on_mouse_down(Point::new(10., 10.));
    o.rectangle_tool.on_mouse_move(Point::new(60., 50.));
    assert!(o.in_progress().is_some());
    // A keyboard shortcut switches tools before the mouse is released.
    o.discard_tool_stroke();
    o.active_tool = ToolKind::Arrow;
    o.active_tool = ToolKind::Rectangle;
    assert!(o.in_progress().is_none());
}
