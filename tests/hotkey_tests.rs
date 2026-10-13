use global_hotkey::hotkey::{Code, Modifiers};
use snipping_tool_ul::hotkey::parse_binding;

#[test]
fn parse_default_binding() {
    let (mods, code) = parse_binding("Ctrl+Shift+S").expect("default binding parses");
    assert!(mods.contains(Modifiers::CONTROL));
    assert!(mods.contains(Modifiers::SHIFT));
    assert_eq!(code, Code::KeyS);
}

#[test]
fn parse_is_case_insensitive_and_trims() {
    let (mods, code) = parse_binding("ctrl + ALT + p").expect("messy binding parses");
    assert!(mods.contains(Modifiers::CONTROL));
    assert!(mods.contains(Modifiers::ALT));
    assert_eq!(code, Code::KeyP);
}

#[test]
fn parse_digits() {
    let (mods, code) = parse_binding("Ctrl+Shift+1").expect("digit binding parses");
    assert!(mods.contains(Modifiers::CONTROL));
    assert_eq!(code, Code::Digit1);

    let (_, code) = parse_binding("Alt+0").expect("digit 0 parses");
    assert_eq!(code, Code::Digit0);
}

#[test]
fn parse_function_keys_and_printscreen() {
    let (_, code) = parse_binding("Super+F5").expect("F-key parses");
    assert_eq!(code, Code::F5);
    let (_, code) = parse_binding("PrintScreen").expect("PrintScreen parses");
    assert_eq!(code, Code::PrintScreen);
}

#[test]
fn parse_rejects_garbage() {
    assert!(parse_binding("").is_err());
    assert!(parse_binding("Ctrl+Shift").is_err()); // modifiers only
    assert!(parse_binding("Ctrl+Banana").is_err());
    assert!(parse_binding("Ctrl+!").is_err());
}
