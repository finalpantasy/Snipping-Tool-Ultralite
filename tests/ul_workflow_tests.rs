use snipping_tool_ul::{
    config::Config,
    geometry::Point,
    launcher::{Action, CaptureMode, Model},
    preview::EditorView,
};

#[test]
fn preferences_keep_options_open_and_outside_click_dismisses_it() {
    use snipping_tool_ul::launcher::Menu;
    let mut m = Model::new(Default::default(), String::new());
    m.activate(Action::ToggleOptions);
    for action in [Action::AutoSave, Action::Preview, Action::ConfirmUpload] {
        let t = m
            .menu_targets(660.)
            .into_iter()
            .find(|t| t.action == action)
            .unwrap();
        assert_eq!(
            m.hit(Point::new(t.rect.x + 10., t.rect.y + 16.)),
            Some(action)
        );
        assert!(!m.activate(action));
        assert_eq!(m.menu, Menu::Options);
    }
    assert_eq!(m.hit(Point::new(20., 270.)), Some(Action::Dismiss));
    assert!(m.activate(Action::Dismiss));
    assert_eq!(m.menu, Menu::None);
    m.activate(Action::ToggleOptions);
    assert!(!m.activate(Action::Settings));
    assert_eq!(m.menu, Menu::None);
}

#[test]
fn upload_prompt_has_explicit_confirm_and_cancel_with_safe_keyboard_default() {
    use snipping_tool_ul::launcher::Menu;
    let mut m = Model::new(Default::default(), String::new());
    m.menu = Menu::UploadConfirm;
    assert_eq!(m.enter(660.), Some(Action::CancelUpload));
    let targets = m.menu_targets(660.);
    for t in &targets {
        assert!(t.rect.y + t.rect.h < snipping_tool_ul::launcher::HEIGHT as f32);
        assert_eq!(
            m.hit(Point::new(t.rect.x + 10., t.rect.y + 10.)),
            Some(t.action)
        );
    }
    m.navigate(1, 660.);
    m.navigate(1, 660.);
    assert_eq!(m.enter(660.), Some(Action::ConfirmPublicUpload));
    assert_eq!(m.hit(Point::new(30., 30.)), Some(Action::Dismiss));
}

#[test]
fn workflow_defaults_keep_immediate_preview_without_disk_or_network_actions() {
    let c: Config = toml::from_str("").unwrap();
    assert!(c.workflow.open_preview);
    assert!(!c.workflow.auto_save);
    assert!(!c.workflow.confirm_upload);
    assert!(!c.general.history_enabled);
    let roundtrip: Config = toml::from_str(&toml::to_string(&c).unwrap()).unwrap();
    assert!(roundtrip.workflow.open_preview);
}
#[test]
fn launcher_targets_disable_upload_without_an_image_and_isolate_connection_modal() {
    let mut m = Model::new(Default::default(), "Ctrl+Shift+S".into());
    assert_eq!(m.hit(Point::new(50., 32.)), Some(Action::New));
    m.activate(Action::ToggleModes);
    assert_eq!(
        m.hit(Point::new(130., 103.)),
        Some(Action::Mode(CaptureMode::Window))
    );
    m.activate(Action::Dismiss);
    assert_eq!(m.hit(Point::new(480., 32.)), None);
    m.has_image = true;
    assert_eq!(m.hit(Point::new(480., 32.)), Some(Action::Upload));
    m.connecting = true;
    assert_eq!(m.hit(Point::new(50., 32.)), None);
    assert_eq!(m.hit(Point::new(120., 249.)), Some(Action::SaveConnection));
    m.append("a\r\nb !$1");
    assert_eq!(m.draft, "ab1");
    m.append(&"x".repeat(150));
    assert_eq!(m.draft.len(), 128);
}
#[test]
fn timer_choices_cycle_and_hit_testing_uses_half_open_bounds() {
    let mut m = Model::new(Default::default(), String::new());
    for expected in [3, 5, 10, 0] {
        m.cycle_delay();
        assert_eq!(m.delay, expected);
    }
    assert_eq!(m.hit(Point::new(102., 32.)), None);
}

#[test]
fn quit_is_visible_in_options_and_reachable_by_mouse_and_keyboard() {
    let mut m = Model::new(Default::default(), String::new());
    m.activate(Action::ToggleOptions);
    let quit = m
        .menu_targets(660.)
        .into_iter()
        .find(|t| t.action == Action::Quit)
        .unwrap();
    assert!(quit.rect.y + quit.rect.h < snipping_tool_ul::launcher::HEIGHT as f32);
    assert_eq!(
        m.hit(Point::new(quit.rect.x + 20., quit.rect.y + 16.)),
        Some(Action::Quit)
    );
    for _ in 0..7 {
        m.navigate(1, 660.);
    }
    assert_eq!(m.enter(660.), Some(Action::Quit));
}
#[test]
fn cursor_anchor_and_footer_are_stable_across_zoom_and_mixed_dpi() {
    for dpi in [1., 1.25, 1.5, 2.] {
        for (iw, ih, vw, vh) in [
            (1918, 401, 1200, 650),
            (200, 100, 984, 400),
            (3840, 2160, 1200, 800),
        ] {
            let mut v = EditorView::default();
            let cw = 984;
            let (x, y) = (vw as f64 * 0.63, vh as f64 * 0.41);
            let before = v.point(x, y, iw, ih, cw, vw, vh, dpi);
            let footer = v.layout(iw, ih, cw, vw, vh, dpi);
            for steps in [2., -1., 4., -3.] {
                v.zoom_at(x, y, steps, iw, ih, cw, vw, vh, dpi);
                let after = v.point(x, y, iw, ih, cw, vw, vh, dpi);
                assert!((before.x - after.x).abs() < 0.001 && (before.y - after.y).abs() < 0.001);
                let new = v.layout(iw, ih, cw, vw, vh, dpi);
                assert_eq!(footer.footer_y, new.footer_y);
                assert_eq!(footer.footer_scale, new.footer_scale);
                let p = v.point(
                    new.footer_x + 24. * new.footer_scale,
                    new.footer_y + 24. * new.footer_scale,
                    iw,
                    ih,
                    cw,
                    vw,
                    vh,
                    dpi,
                );
                assert!((p.x - 24.).abs() < 0.001 && (p.y - (ih as f32 + 24.)).abs() < 0.001);
            }
            let zoom = v.zoom;
            v.zoom_at(20., vh as f64 - 2., 5., iw, ih, cw, vw, vh, dpi);
            assert_eq!(zoom, v.zoom);
        }
    }
}
#[test]
fn presentation_zoom_preserves_source_and_fixed_footer_pixels() {
    let mut p = tiny_skia::Pixmap::new(100, 164).unwrap();
    p.fill(tiny_skia::Color::from_rgba8(255, 0, 0, 255));
    for y in 100..164 {
        for x in 0..100 {
            p.data_mut()[((y * 100 + x) * 4) as usize..((y * 100 + x) * 4 + 4) as usize]
                .copy_from_slice(&[0, 255, 0, 255]);
        }
    }
    let original = p.data().to_vec();
    let mut output = vec![0; 100 * 200];
    let mut v = EditorView::default();
    v.zoom_at(40., 40., 3., 100, 100, 100, 100, 200, 1.);
    v.present(&p, &mut output, 100, 200, 100, 100, 100, 1.);
    assert_eq!(p.data(), original);
    assert_eq!(output[199 * 100 + 50], 0x00ff00);
    assert_eq!(output[40 * 100 + 40], 0xff0000);
}
#[test]
fn invalid_image_is_rejected_before_opening_system_clipboard() {
    assert!(snipping_tool_ul::export::copy_to_clipboard(&[], 0, 0).is_err());
    assert!(snipping_tool_ul::export::copy_to_clipboard(&[0; 3], 1, 1).is_err());
    assert!(snipping_tool_ul::export::validate_image(&[0; 4], 1, 1).is_ok());
    assert!(snipping_tool_ul::export::validate_image(&[], u32::MAX, u32::MAX).is_err());
}
#[test]
fn window_point_selection_uses_front_to_back_order_without_signed_overflow() {
    let windows = [
        (-200, 0, 100, 100),
        (-200, 0, 200, 200),
        (i32::MAX - 4, 0, 100, 100),
    ];
    assert_eq!(
        snipping_tool_ul::window_detect::window_at_point(&windows, -150, 20),
        Some(windows[0])
    );
    assert_eq!(
        snipping_tool_ul::window_detect::window_at_point(&windows, i32::MAX, 20),
        Some(windows[2])
    );
}

#[test]
fn large_preview_toolbar_stays_inside_the_independent_footer() {
    let cfg = Config::default();
    let screen = snipping_tool_ul::capture::CapturedScreen {
        pixels: vec![255; 1920 * 1080 * 4],
        width: 1920,
        height: 1080,
        x_offset: 0,
        y_offset: 0,
        scale_factor: 1.,
    };
    let mut state = snipping_tool_ul::state::OverlayState::new(screen, &cfg);
    state.preview_mode = true;
    let sel = snipping_tool_ul::overlay::selection::Selection {
        x: 0.,
        y: 0.,
        width: 1920.,
        height: 1080.,
    };
    let toolbar = state.toolbar(&sel);
    let cw = snipping_tool_ul::preview::controls_width(state.visible_buttons.len());
    assert_eq!(toolbar.x, 8.);
    assert_eq!(toolbar.y, 1088.);
    assert!(toolbar.x + toolbar.width <= cw as f32);
    for i in 0..state.visible_buttons.len() {
        let (x, y, w, h) = toolbar.button_rect(i);
        assert_eq!(
            toolbar.hit_test_dynamic(
                Point::new(x + w / 2., y + h / 2.),
                state.visible_buttons.len()
            ),
            Some(i)
        );
    }
}

#[test]
fn launcher_rasterizes_controls_at_physical_display_scale() {
    let model = Model::new(Default::default(), "Ctrl+Shift+S".into());
    for scale in [1., 1.25, 1.5, 2.] {
        let w = (snipping_tool_ul::launcher::WIDTH as f32 * scale) as u32;
        let h = (snipping_tool_ul::launcher::HEIGHT as f32 * scale) as u32;
        let mut p = tiny_skia::Pixmap::new(w, h).unwrap();
        model.render(&mut p);
        // Interior of New stays teal and body transparency survives physical DPI scaling.
        let (x, y) = ((24. * scale) as usize, (30. * scale) as usize);
        let pixel = &p.data()[(y * w as usize + x) * 4..(y * w as usize + x) * 4 + 4];
        assert_eq!(pixel, &[0, 124, 145, 255]);
        assert_eq!(
            p.data()[p.data().len() - 1],
            snipping_tool_ul::theme::glass_alpha()
        );
    }
}

#[test]
fn empty_launcher_is_compact_and_menus_expand_then_collapse() {
    use snipping_tool_ul::launcher::{COMPACT_HEIGHT, HEIGHT, WIDTH};
    let mut m = Model::new(Default::default(), "Ctrl+Shift+S".into());
    assert_eq!(m.height(), COMPACT_HEIGHT);
    assert!(COMPACT_HEIGHT < HEIGHT / 2);
    for toggle in [
        Action::ToggleOptions,
        Action::ToggleModes,
        Action::ToggleDelay,
    ] {
        assert!(m.activate(toggle));
        assert_eq!(m.height(), HEIGHT);
        for target in m.menu_targets(WIDTH as f32) {
            assert!(target.rect.y + target.rect.h < m.height() as f32);
            assert_eq!(
                m.hit(Point::new(target.rect.x + 8., target.rect.y + 8.)),
                Some(target.action)
            );
        }
        assert!(m.activate(Action::Dismiss));
        assert_eq!(m.height(), COMPACT_HEIGHT);
    }
    m.connecting = true;
    assert_eq!(m.height(), HEIGHT);
    m.connecting = false;
    assert_eq!(m.height(), COMPACT_HEIGHT);
}

#[test]
fn compact_launcher_keeps_new_and_keyboard_navigation_reachable() {
    use snipping_tool_ul::launcher::{COMPACT_HEIGHT, WIDTH};
    let mut m = Model::new(Default::default(), "Alt+PrintScreen".into());
    for target in m.bar_targets(WIDTH as f32, false) {
        assert!(target.rect.y + target.rect.h < COMPACT_HEIGHT as f32);
        assert_eq!(
            m.hit(Point::new(target.rect.x + 8., target.rect.y + 8.)),
            Some(target.action)
        );
    }
    assert_eq!(m.enter(WIDTH as f32), Some(Action::New));
    m.navigate(1, WIDTH as f32);
    assert_eq!(m.enter(WIDTH as f32), Some(Action::New));
    m.navigate(1, WIDTH as f32);
    assert_eq!(m.enter(WIDTH as f32), Some(Action::ToggleModes));
    assert_eq!(m.hotkey, "Alt+PrintScreen");
}
