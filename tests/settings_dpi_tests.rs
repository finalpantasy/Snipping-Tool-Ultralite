use snipping_tool_ul::{
    config::Config,
    settings_ui::{Action, SettingsUi, WIN_H, WIN_W},
};

#[test]
fn settings_text_has_native_high_dpi_detail_instead_of_enlarged_pixels() {
    let mut ui = SettingsUi::new(Config::default());
    let mut base = tiny_skia::Pixmap::new(WIN_W, WIN_H).unwrap();
    ui.draw(&mut base);
    let mut high = tiny_skia::Pixmap::new(WIN_W * 2, WIN_H * 2).unwrap();
    ui.draw(&mut high);
    let mut differences = 0;
    // The Settings title, excluding its icon and any other controls.
    for y in 40..80usize {
        for x in 112..240usize {
            let a = (y * (WIN_W * 2) as usize + x) * 4;
            let b = ((y / 2) * WIN_W as usize + x / 2) * 4;
            differences += usize::from(high.data()[a..a + 4] != base.data()[b..b + 4]);
        }
    }
    assert!(
        differences > 100,
        "Text must be rasterized at physical resolution"
    );
}

#[test]
fn settings_tabs_and_tool_toggles_keep_logical_hit_targets_at_every_dpi() {
    for dpi in [1., 1.25, 1.5, 2.] {
        let mut ui = SettingsUi::new(Config::default());
        let mut p =
            tiny_skia::Pixmap::new((WIN_W as f32 * dpi) as u32, (WIN_H as f32 * dpi) as u32)
                .unwrap();
        ui.active_tab = 2;
        ui.draw(&mut p);
        let Some(Action::ToggleToolbar(i)) = ui.on_click(380., 136.) else {
            panic!("First tool toggle");
        };
        let before = ui.config.toolbar.visible_button_indices();
        ui.handle_action(Action::ToggleToolbar(i));
        assert_ne!(before, ui.config.toolbar.visible_button_indices());
        assert!(matches!(ui.on_click(365., 65.), Some(Action::SwitchTab(3))));
        assert!(matches!(
            ui.on_click(330., snipping_tool_ul::settings_ui::WIN_H as f32 - 40.),
            Some(Action::SaveClose)
        ));
    }
}

#[test]
fn close_behavior_setting_is_reachable_and_toggles_without_external_actions() {
    let mut ui = SettingsUi::new(Config::default());
    let mut p = tiny_skia::Pixmap::new(WIN_W, WIN_H).unwrap();
    ui.draw(&mut p);
    let mut target = None;
    for y in 105..WIN_H - 60 {
        if matches!(
            ui.on_click(330., y as f32),
            Some(Action::SetCloseBehavior(true))
        ) {
            target = ui.on_click(330., y as f32);
            break;
        }
    }
    assert!(!ui.config.general.quit_on_close);
    assert!(!ui.handle_action(target.expect("Close behavior must be visible above Save & Close")));
    assert!(ui.config.general.quit_on_close);
    ui.handle_action(Action::SetCloseBehavior(true));
    assert!(ui.config.general.quit_on_close);
    ui.handle_action(Action::SetCloseBehavior(false));
    assert!(!ui.config.general.quit_on_close);
}
