use snipping_tool_ul::{
    capture::CapturedScreen,
    config::Config,
    editor_ui::EditorUi,
    geometry::Point,
    launcher::{Action, Menu, Model},
    overlay::selection::Selection,
    preview::EditorView,
    state::OverlayState,
};
fn state() -> OverlayState {
    let mut o = OverlayState::new(
        CapturedScreen {
            pixels: vec![255; 640 * 354 * 4],
            width: 640,
            height: 354,
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
        width: 640.,
        height: 354.,
    });
    o
}
#[test]
fn every_existing_annotation_and_export_action_is_reachable() {
    let o = state();
    let mut ui = EditorUi::new(&Config::default());
    let mut found = Vec::new();
    for menu in [Menu::None, Menu::Tools, Menu::Colors] {
        ui.header.menu = menu;
        for y in (0..612).step_by(8) {
            for x in (0..940).step_by(8) {
                if let Some(Action::Button(i)) =
                    ui.hit(Point::new(x as f32, y as f32), 940., 612., &o)
                {
                    if !found.contains(&i) {
                        found.push(i);
                    }
                }
            }
        }
    }
    // Upload has its own action label rather than an annotation button index.
    ui.header.menu = Menu::None;
    assert_eq!(
        ui.hit(Point::new(838., 30.), 940., 612., &o),
        Some(Action::Upload)
    );
    for i in (0..24).filter(|i| *i != 20) {
        assert!(found.contains(&i), "Missing original toolbar action {i}");
    }
    ui.header.menu = Menu::Colors;
    assert!(matches!(
        ui.hit(Point::new(600., 499.), 940., 612., &o),
        Some(Action::CustomColor)
    ));
}
#[test]
fn menu_clicks_cannot_reach_the_image_and_keyboard_navigation_wraps() {
    let o = state();
    let mut ui = EditorUi::new(&Config::default());
    ui.header.menu = Menu::Modes;
    assert_eq!(
        ui.hit(Point::new(500., 300.), 940., 612., &o),
        Some(Action::Dismiss)
    );
    ui.navigate(940., 612., &o, 1);
    assert_eq!(
        ui.enter(940., 612., &o),
        Some(Action::Mode(
            snipping_tool_ul::launcher::CaptureMode::Region
        ))
    );
    ui.navigate(940., 612., &o, -1);
    assert_eq!(
        ui.enter(940., 612., &o),
        Some(Action::Mode(
            snipping_tool_ul::launcher::CaptureMode::FullScreen
        ))
    );
    assert!(ui.activate(Action::Dismiss));
    assert_eq!(ui.header.menu, Menu::None);
    ui.navigate(940., 612., &o, 1);
    assert!(ui.header.keyboard_focus);
    assert!(ui.hover(Point::new(45., 30.), 940., 612., &o));
    assert!(!ui.header.keyboard_focus);
    assert!(!ui.hover(Point::new(45., 30.), 940., 612., &o));
    let mut home = Model::new(Default::default(), "Ctrl+Shift+S".into());
    home.navigate(1, 660.);
    assert!(home.hover(Point::new(45., 30.)));
    assert!(!home.keyboard_focus);
    assert!(!home.hover(Point::new(45., 30.)));
}
#[test]
fn framed_zoom_anchors_cursor_and_excludes_controls_across_dpi() {
    for dpi in [1., 1.25, 1.5, 2.] {
        let (w, h) = ((940. * dpi) as u32, (612. * dpi) as u32);
        let mut v = EditorView::default();
        let (x, y) = (530. * dpi, 230. * dpi);
        let before = v.framed_point(x, y, 3840, 2160, w, h, dpi);
        v.zoom_framed(x, y, 3., 3840, 2160, w, h, dpi);
        let after = v.framed_point(x, y, 3840, 2160, w, h, dpi);
        assert!((before.x - after.x).abs() < 0.001 && (before.y - after.y).abs() < 0.001);
        let zoom = v.zoom;
        v.zoom_framed(50. * dpi, 30. * dpi, 3., 3840, 2160, w, h, dpi);
        assert_eq!(zoom, v.zoom);
        v.zoom_framed(300. * dpi, 550. * dpi, 3., 3840, 2160, w, h, dpi);
        assert_eq!(zoom, v.zoom);
    }
}
#[test]
fn source_pixels_and_ui_regions_stay_separate_during_zoom() {
    let mut source = tiny_skia::Pixmap::new(640, 354).unwrap();
    source.fill(tiny_skia::Color::from_rgba8(255, 0, 0, 255));
    let original = source.data().to_vec();
    let ui = EditorUi::new(&Config::default());
    let mut frame = tiny_skia::Pixmap::new(940, 612).unwrap();
    ui.draw_base(&mut frame, 1.);
    let header = frame.data()[30 * 940 * 4..31 * 940 * 4].to_vec();
    let footer = frame.data()[555 * 940 * 4..556 * 940 * 4].to_vec();
    let mut v = EditorView::default();
    v.zoom_framed(520., 200., 8., 640, 354, 940, 612, 1.);
    v.paint_framed(&source, &mut frame, 1.);
    assert_eq!(source.data(), original);
    assert_eq!(&frame.data()[30 * 940 * 4..31 * 940 * 4], header);
    assert_eq!(&frame.data()[555 * 940 * 4..556 * 940 * 4], footer);
}
#[test]
fn text_commit_is_included_in_export_and_reversible_without_extra_undo_steps() {
    let mut o = state();
    o.text_input_active = true;
    o.text_input_position = Point::new(10., 10.);
    o.text_input_buffer = "Capture note".into();
    o.commit_text();
    assert!(!o.text_input_active);
    assert_eq!(o.annotations.len(), 1);
    assert_eq!(o.undo_stack.len(), 1);
    let pixels = o.flattened_selection().unwrap().0;
    assert_ne!(pixels, o.screenshot.pixels);
    assert!(snipping_tool_ul::tools::apply_undo(
        &mut o.annotations,
        &mut o.undo_stack,
        &mut o.redo_stack
    ));
    assert!(o.annotations.is_empty());
    assert!(snipping_tool_ul::tools::apply_redo(
        &mut o.annotations,
        &mut o.undo_stack,
        &mut o.redo_stack
    ));
    assert_eq!(o.annotations.len(), 1);
}
#[test]
fn glass_and_glyph_pixels_are_premultiplied_and_fallback_keeps_channel_order() {
    let mut p = tiny_skia::Pixmap::new(660, 372).unwrap();
    let m = Model::new(Default::default(), "Ctrl+Shift+S".into());
    m.render(&mut p);
    // Header is independent of backdrop brightness and matches the native caption token.
    let base = snipping_tool_ul::ui::base();
    assert_eq!(
        &p.data()[(2 * 660 + 2) * 4..(2 * 660 + 2) * 4 + 4],
        &[base.0, base.1, base.2, 255]
    );
    assert!(p
        .data()
        .chunks_exact(4)
        .all(|px| px[..3].iter().all(|c| *c <= px[3])));
    assert_eq!(
        p.data()[p.data().len() - 1],
        snipping_tool_ul::theme::glass_alpha()
    );
    assert_eq!(
        snipping_tool_ul::ui::opaque_rgb(&[255, 0, 0, 255, 0, 0, 0, 0]),
        vec![0xff0000, 0x142025]
    );
}
#[test]
fn source_renderer_does_not_bake_frame_controls_into_capture() {
    let mut o = state();
    let mut p = tiny_skia::Pixmap::new(640, 354).unwrap();
    snipping_tool_ul::renderer::render_overlay(&mut o, &mut p);
    assert_eq!(p.data(), o.screenshot.pixels);
}

#[test]
fn organized_controls_remain_reachable_at_minimum_size_with_no_overlap() {
    let o = state();
    let mut ui = EditorUi::new(&Config::default());
    for (w, h) in [(660., 390.), (940., 612.)] {
        let mut actions = Vec::new();
        for menu in [Menu::None, Menu::Tools, Menu::Colors] {
            ui.header.menu = menu;
            for y in (0..h as u32).step_by(4) {
                for x in (0..w as u32).step_by(4) {
                    if let Some(a) = ui.hit(Point::new(x as f32, y as f32), w, h, &o) {
                        if !actions.contains(&a) {
                            actions.push(a);
                        }
                    }
                }
            }
        }
        for i in (0..24).filter(|i| *i != 20) {
            assert!(
                actions.contains(&Action::Button(i)),
                "Missing {i} at {w}x{h}"
            );
        }
        ui.header.menu = Menu::None;
        // The text below the icon activates the same tool, rather than drawing on the image.
        assert_eq!(
            ui.hit(Point::new((w - 612.) / 2. + 120., h - 47.), w, h, &o),
            Some(Action::Button(6))
        );
        let t = ui.header.bar_targets(w, true);
        for (i, a) in t.iter().enumerate() {
            for b in &t[i + 1..] {
                assert!(a.rect.x + a.rect.w <= b.rect.x || b.rect.x + b.rect.w <= a.rect.x);
            }
        }
    }
    ui.header.menu = Menu::Options;
    let targets = ui.header.menu_targets(660.);
    assert!(targets.last().unwrap().rect.y + 32. < snipping_tool_ul::launcher::HEIGHT as f32);
    assert_eq!(
        ui.hit(Point::new(400., 150.), 660., 390., &o),
        Some(Action::Dismiss)
    );
    for t in targets {
        assert_eq!(
            ui.hit(Point::new(t.rect.x + 12., t.rect.y + 16.), 660., 390., &o),
            Some(t.action)
        );
    }
}
