use clap::Parser;
use snipping_tool_ul::{
    cli::{Cli, Commands},
    geometry::Point,
    windows_integration::{RESTORE_BUTTON, TAKEOVER_BUTTON},
};

#[test]
fn install_defaults_never_apply_or_replace_windows() {
    let cli = Cli::try_parse_from(["ul", "install"]).unwrap();
    assert!(matches!(
        cli.command,
        Some(Commands::Install {
            apply: false,
            replace_windows_snipping_tool: false,
            interactive: false
        })
    ));
    let cli = Cli::try_parse_from(["ul", "install", "--replace-windows-snipping-tool"]).unwrap();
    assert!(matches!(
        cli.command,
        Some(Commands::Install {
            apply: false,
            replace_windows_snipping_tool: true,
            ..
        })
    ));
    assert!(Cli::try_parse_from(["ul", "install", "--interactive"]).is_err());
    assert!(Cli::try_parse_from(["ul", "install", "--apply", "--interactive"]).is_ok());
}
#[test]
fn windows_actions_have_distinct_bounded_targets() {
    assert!(TAKEOVER_BUTTON.contains(Point::new(100., 185.)));
    assert!(!RESTORE_BUTTON.contains(Point::new(100., 185.)));
    assert!(RESTORE_BUTTON.contains(Point::new(100., 325.)));
    assert!(!TAKEOVER_BUTTON.contains(Point::new(100., 325.)));
    for r in [TAKEOVER_BUTTON, RESTORE_BUTTON] {
        assert!(r.x >= 24. && r.x + r.w <= 396. && r.y + r.h < 466.);
    }
}
