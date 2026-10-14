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

#[test]
fn recorder_uses_physical_keys_so_shifted_digits_and_azerty_work() {
    use snipping_tool_ul::hotkey::binding_from_key;
    use winit::keyboard::{KeyCode, ModifiersState, PhysicalKey};
    let ctrl_shift = ModifiersState::CONTROL | ModifiersState::SHIFT;
    // Shift+1 types "!" on US layouts and AZERTY's top row types symbols.
    assert_eq!(
        binding_from_key(ctrl_shift, &PhysicalKey::Code(KeyCode::Digit1)).as_deref(),
        Some("Ctrl+Shift+1")
    );
    assert_eq!(
        binding_from_key(ctrl_shift, &PhysicalKey::Code(KeyCode::KeyS)).as_deref(),
        Some("Ctrl+Shift+S")
    );
    assert_eq!(
        binding_from_key(ModifiersState::ALT, &PhysicalKey::Code(KeyCode::F7)).as_deref(),
        Some("Alt+F7")
    );
    // A strong modifier is still required, and other keys are refused.
    assert_eq!(
        binding_from_key(ModifiersState::SHIFT, &PhysicalKey::Code(KeyCode::KeyS)),
        None
    );
    assert_eq!(
        binding_from_key(ctrl_shift, &PhysicalKey::Code(KeyCode::Space)),
        None
    );
}
