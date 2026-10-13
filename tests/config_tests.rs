use snipping_tool_ul::config::Config;

#[test]
fn auto_save_honors_a_nonexistent_absolute_folder_instead_of_falling_back() {
    let mut cfg = Config::default();
    let folder = std::env::temp_dir().join("ul-nonexistent-save-destination");
    cfg.general.save_directory = folder.to_string_lossy().into_owned();
    assert_eq!(cfg.auto_save_directory().unwrap(), folder);
    cfg.general.save_directory = "relative-folder".into();
    assert!(cfg.auto_save_directory().unwrap_err().contains("absolute"));
    cfg.general.save_directory.clear();
    if let Some(pictures) = dirs::picture_dir() {
        assert_eq!(
            cfg.auto_save_directory().unwrap(),
            pictures.join("Snipping Tool UL")
        );
    }
}

#[test]
fn close_behavior_preserves_old_configs_and_explicit_choice() {
    let old: Config = toml::from_str("").unwrap();
    assert!(!old.general.quit_on_close);
    let chosen: Config = toml::from_str("[general]\nquit_on_close = true").unwrap();
    assert!(chosen.general.quit_on_close);
    let saved: Config = toml::from_str(&toml::to_string(&chosen).unwrap()).unwrap();
    assert!(saved.general.quit_on_close);
}
use snipping_tool_ul::geometry::Color;

#[test]
fn default_config_has_correct_values() {
    let cfg = Config::default();
    assert_eq!(cfg.general.default_color, "red");
    assert_eq!(cfg.general.default_thickness, 3.0);
    assert_eq!(cfg.general.save_directory, "");
    assert_eq!(cfg.hotkey.capture, "Ctrl+Shift+S");
}

#[test]
fn parse_color_valid() {
    let mut cfg = Config::default();
    cfg.general.default_color = "blue".to_string();
    assert_eq!(cfg.default_color(), Color::blue());

    cfg.general.default_color = "green".to_string();
    assert_eq!(cfg.default_color(), Color::green());

    cfg.general.default_color = "yellow".to_string();
    assert_eq!(cfg.default_color(), Color::yellow());

    cfg.general.default_color = "white".to_string();
    assert_eq!(cfg.default_color(), Color::white());

    cfg.general.default_color = "red".to_string();
    assert_eq!(cfg.default_color(), Color::red());
}

#[test]
fn parse_color_invalid_falls_back_to_red() {
    let mut cfg = Config::default();
    cfg.general.default_color = "magenta".to_string();
    assert_eq!(cfg.default_color(), Color::red());

    cfg.general.default_color = "".to_string();
    assert_eq!(cfg.default_color(), Color::red());
}

#[test]
fn serialize_roundtrip_via_toml() {
    let cfg = Config::default();
    let serialized = toml::to_string(&cfg).expect("serialize");
    let deserialized: Config = toml::from_str(&serialized).expect("deserialize");
    assert_eq!(
        deserialized.general.default_color,
        cfg.general.default_color
    );
    assert_eq!(
        deserialized.general.default_thickness,
        cfg.general.default_thickness
    );
    assert_eq!(
        deserialized.general.save_directory,
        cfg.general.save_directory
    );
    assert_eq!(deserialized.hotkey.capture, cfg.hotkey.capture);
}

#[test]
fn partial_config_parses_with_defaults() {
    // A config missing keys/sections (older version, hand-edited) must parse
    // instead of silently resetting everything to defaults.
    let toml_src = r#"
[general]
default_color = "blue"

[shortcuts]
arrow = "q"
"#;
    let cfg: Config = toml::from_str(toml_src).expect("partial config should parse");
    assert_eq!(cfg.general.default_color, "blue");
    assert_eq!(cfg.general.default_thickness, 3.0); // default filled in
    assert!(!cfg.general.history_enabled); // disk history is opt-in in the lightweight fork
    assert_eq!(cfg.hotkey.capture, "Ctrl+Shift+S"); // whole section defaulted
    assert_eq!(cfg.shortcuts.arrow, "q");
    assert_eq!(cfg.shortcuts.rectangle, "r"); // sibling key defaulted
    assert!(cfg.toolbar.select);
}

#[test]
fn empty_config_parses_as_defaults() {
    let cfg: Config = toml::from_str("").expect("empty config should parse");
    assert_eq!(cfg.general.default_color, "red");
    assert_eq!(cfg.hotkey.capture, "Ctrl+Shift+S");
}

#[test]
fn parse_color_hex() {
    let mut cfg = Config::default();
    cfg.general.default_color = "#6bdcff".to_string();
    let c = cfg.default_color();
    assert!((c.r - 0x6b as f32 / 255.0).abs() < 0.001);
    assert!((c.g - 0xdc as f32 / 255.0).abs() < 0.001);
    assert!((c.b - 0xff as f32 / 255.0).abs() < 0.001);

    // Malformed hex falls back to red
    cfg.general.default_color = "#zzzzzz".to_string();
    assert_eq!(cfg.default_color(), Color::red());
    cfg.general.default_color = "#fff".to_string();
    assert_eq!(cfg.default_color(), Color::red());
}

#[test]
fn thickness_clamping() {
    let mut cfg = Config::default();

    cfg.general.default_thickness = 100.0;
    assert_eq!(cfg.clamped_thickness(), 20.0);

    cfg.general.default_thickness = -5.0;
    assert_eq!(cfg.clamped_thickness(), 1.0);

    cfg.general.default_thickness = 10.0;
    assert_eq!(cfg.clamped_thickness(), 10.0);
}

#[test]
fn theme_defaults_to_light_and_preserves_saved_dark_choice() {
    let cfg = snipping_tool_ul::config::Config::default();
    assert_eq!(cfg.general.theme, "light");
    assert_eq!(cfg.theme_mode(), snipping_tool_ul::theme::ThemeMode::Light);
    let missing: Config = toml::from_str("[general]\nsave_directory = ''").unwrap();
    assert_eq!(
        missing.theme_mode(),
        snipping_tool_ul::theme::ThemeMode::Light
    );
    let saved: Config = toml::from_str("[general]\ntheme = 'DaRk'").unwrap();
    assert_eq!(saved.theme_mode(), snipping_tool_ul::theme::ThemeMode::Dark);
}

#[test]
fn theme_mode_parses_light_case_insensitively() {
    let mut cfg = snipping_tool_ul::config::Config::default();
    cfg.general.theme = "Light".to_string();
    assert_eq!(cfg.theme_mode(), snipping_tool_ul::theme::ThemeMode::Light);
    cfg.general.theme = "garbage".to_string();
    assert_eq!(cfg.theme_mode(), snipping_tool_ul::theme::ThemeMode::Light);
}
