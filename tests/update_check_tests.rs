//! Update check logic, with authored replies only. No network access.
use snipping_tool_ul::update_check::{
    check_due, is_newer, newest_release, parse_version, Release, CHECK_INTERVAL_SECS,
};

#[test]
fn versions_compare_numerically_and_ignore_tag_suffixes() {
    assert_eq!(parse_version("v0.3.10-preview"), Some(vec![0, 3, 10]));
    assert_eq!(parse_version("0.3.23"), Some(vec![0, 3, 23]));
    assert_eq!(parse_version("latest"), None);
    assert!(is_newer("0.3.10", "0.3.9"));
    assert!(is_newer("v0.3.24-preview", "0.3.23"));
    assert!(is_newer("0.4", "0.3.99"));
    assert!(is_newer("1.0.0", "0.9.9"));
    assert!(!is_newer("0.3.9", "0.3.10"));
    assert!(!is_newer("v0.3.23-preview", "0.3.23"));
    assert!(!is_newer("0.3.23.0", "0.3.23"));
    assert!(!is_newer("garbage", "0.3.23"));
}

#[test]
fn checks_at_most_once_every_seven_days() {
    let week = CHECK_INTERVAL_SECS;
    let start = 1_800_000_000;
    // Never checked before.
    assert!(check_due(0, start));
    assert!(!check_due(start, start));
    assert!(!check_due(start, start + week - 1));
    assert!(check_due(start, start + week));
    // A clock set back must not block checks for ever.
    assert!(check_due(start + 3600, start));
}

#[test]
fn picks_the_newest_published_release_including_prereleases() {
    let reply = r#"[
        {"tag_name": "v0.3.24-preview", "draft": false, "prerelease": true,
         "html_url": "https://github.com/finalpantasy/Snipping-Tool-Ultralite/releases/tag/v0.3.24-preview"},
        {"tag_name": "v0.3.30", "draft": true,
         "html_url": "https://github.com/finalpantasy/Snipping-Tool-Ultralite/releases/tag/v0.3.30"},
        {"tag_name": "v0.3.9", "draft": false,
         "html_url": "https://github.com/finalpantasy/Snipping-Tool-Ultralite/releases/tag/v0.3.9"},
        {"tag_name": "v9.9.9", "draft": false,
         "html_url": "https://example.com/not-our-page"}
    ]"#;
    assert_eq!(
        newest_release(reply, "0.3.23"),
        Some(Release {
            version: "0.3.24".into(),
            url: "https://github.com/finalpantasy/Snipping-Tool-Ultralite/releases/tag/v0.3.24-preview".into(),
        })
    );
    // Nothing newer, or an unreadable reply, offers nothing.
    assert_eq!(newest_release(reply, "0.3.24"), None);
    assert_eq!(
        newest_release("{\"message\": \"rate limited\"}", "0.3.23"),
        None
    );
    assert_eq!(newest_release("not json", "0.3.23"), None);
}

#[test]
fn only_this_apps_release_pages_can_be_opened() {
    assert!(snipping_tool_ul::update_check::open_release_page("https://example.com/").is_err());
}

#[test]
fn home_offers_the_update_link_only_when_idle() {
    use snipping_tool_ul::geometry::Point;
    use snipping_tool_ul::launcher::{Action, Model};
    let mut m = Model::new(Default::default(), "Ctrl+Shift+S".into());
    assert!(m.update_link().is_none());
    m.update = Some("0.3.24".into());
    let (r, text) = m.update_link().unwrap();
    assert_eq!(text, "Update available: 0.3.24");
    let centre = Point::new(r.x + r.w / 2., r.y + r.h / 2.);
    assert_eq!(m.hit(centre), Some(Action::OpenUpdate));
    // A real status message takes the place instead, and the link stops working.
    m.status = "Copied to clipboard".into();
    assert!(m.update_link().is_none());
    assert_ne!(m.hit(centre), Some(Action::OpenUpdate));
}

#[test]
fn settings_switch_turns_the_weekly_check_off_and_on() {
    use snipping_tool_ul::config::Config;
    use snipping_tool_ul::settings_ui::{Action, SettingsUi, WIN_H, WIN_W};
    let config = Config::default();
    assert!(config.general.check_updates, "on by default");
    let mut ui = SettingsUi::new(config);
    let mut frame = tiny_skia::Pixmap::new(WIN_W, WIN_H).unwrap();
    ui.draw(&mut frame);
    // Fifth row of the App section.
    let action = ui.on_click(60., 480.);
    assert!(
        matches!(action, Some(Action::ToggleUpdateCheck)),
        "{action:?}"
    );
    ui.handle_action(Action::ToggleUpdateCheck);
    assert!(!ui.config.general.check_updates);
    // Older config files without the keys keep checking, starting fresh.
    let old: Config = toml::from_str("[general]\ntheme = \"dark\"\n").unwrap();
    assert!(old.general.check_updates);
    assert_eq!(old.general.last_update_check, 0);
}
