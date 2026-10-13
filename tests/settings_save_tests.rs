use snipping_tool_ul::{
    config::Config,
    settings_ui::{Action, SettingsUi},
};
#[test]
fn failed_save_keeps_settings_open_with_edits_and_an_explanation() {
    let root = std::env::temp_dir().join(format!("ul-failed-settings-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let blocked = root.join("file-not-directory");
    std::fs::write(&blocked, b"synthetic").unwrap();
    std::env::set_var("SNIPPING_TOOL_UL_CONFIG_DIR", &blocked);
    let mut ui = SettingsUi::new(Config::default());
    ui.config.general.history_enabled = true;
    assert!(!ui.handle_action(Action::SaveClose));
    assert!(ui.config.general.history_enabled);
    assert!(ui
        .save_error
        .as_ref()
        .is_some_and(|s| s.contains("Could not save")));
    std::env::set_var("SNIPPING_TOOL_UL_CONFIG_DIR", root.join("valid"));
    assert!(ui.handle_action(Action::SaveClose));
    assert!(ui.save_error.is_none());
}
