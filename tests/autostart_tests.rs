use snipping_tool_ul::autostart::keeps_newer_registration;
use std::path::{Path, PathBuf};

fn exe(root: &str, version: &str) -> PathBuf {
    Path::new(root).join(version).join("snipping-tool-ul.exe")
}

fn command(path: &Path) -> String {
    format!("\"{}\" --background", path.display())
}

const ROOT: &str = r"C:\Users\Example\AppData\Local\Programs\SnippingToolUL";

#[test]
fn older_instance_keeps_newer_installed_registration() {
    let newer = exe(ROOT, "0.3.22");
    let current = exe(ROOT, "0.3.14");
    assert!(keeps_newer_registration(&command(&newer), &current, |_| {
        true
    }));
}

#[test]
fn newer_instance_replaces_older_registration() {
    let older = exe(ROOT, "0.3.14");
    let current = exe(ROOT, "0.3.22");
    assert!(!keeps_newer_registration(
        &command(&older),
        &current,
        |_| true
    ));
}

#[test]
fn versions_compare_numerically_not_lexically() {
    let newer = exe(ROOT, "0.3.10");
    let current = exe(ROOT, "0.3.9");
    assert!(keeps_newer_registration(&command(&newer), &current, |_| {
        true
    }));
}

#[test]
fn missing_newer_executable_is_replaced() {
    let newer = exe(ROOT, "0.3.22");
    let current = exe(ROOT, "0.3.14");
    assert!(!keeps_newer_registration(
        &command(&newer),
        &current,
        |_| false
    ));
}

#[test]
fn same_version_is_rewritten() {
    let same = exe(ROOT, "0.3.22");
    assert!(!keeps_newer_registration(&command(&same), &same, |_| true));
}

#[test]
fn registration_outside_install_root_is_rewritten() {
    let other = exe(r"D:\Portable\SnippingToolUL", "0.9.0");
    let current = exe(ROOT, "0.3.22");
    assert!(!keeps_newer_registration(
        &command(&other),
        &current,
        |_| true
    ));
}

#[test]
fn portable_current_exe_never_blocks() {
    let newer = exe(ROOT, "0.3.22");
    let portable = Path::new(r"D:\Downloads\snipping-tool-ul.exe");
    assert!(!keeps_newer_registration(
        &command(&newer),
        portable,
        |_| true
    ));
}

#[test]
fn malformed_or_custom_commands_are_rewritten() {
    let current = exe(ROOT, "0.3.14");
    for value in [
        "",
        "snipping-tool-ul.exe --background",
        &format!("\"{}\"", exe(ROOT, "0.3.22").display()),
        &format!("\"{}\" --background --extra", exe(ROOT, "0.3.22").display()),
    ] {
        assert!(
            !keeps_newer_registration(value, &current, |_| true),
            "{value}"
        );
    }
}

#[test]
fn root_comparison_ignores_case() {
    let newer = exe(&ROOT.to_uppercase(), "0.3.22");
    let current = exe(ROOT, "0.3.14");
    assert!(keeps_newer_registration(&command(&newer), &current, |_| {
        true
    }));
}
