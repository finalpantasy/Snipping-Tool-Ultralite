use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use global_hotkey::GlobalHotKeyManager;

/// Parse a binding string like "Ctrl+Shift+S" into modifiers and a key code.
///
/// Supported keys: letters, digits, F1-F12, and PrintScreen.
pub fn parse_binding(binding: &str) -> Result<(Modifiers, Code), String> {
    let parts: Vec<&str> = binding.split('+').collect();
    if parts.is_empty() {
        return Err("Empty hotkey binding".into());
    }

    let mut modifiers = Modifiers::empty();
    let mut key_code: Option<Code> = None;

    for part in &parts {
        match part.trim().to_lowercase().as_str() {
            "ctrl" | "control" => modifiers |= Modifiers::CONTROL,
            "shift" => modifiers |= Modifiers::SHIFT,
            "alt" => modifiers |= Modifiers::ALT,
            "super" | "meta" | "win" => modifiers |= Modifiers::SUPER,
            "printscreen" => key_code = Some(Code::PrintScreen),
            "f1" => key_code = Some(Code::F1),
            "f2" => key_code = Some(Code::F2),
            "f3" => key_code = Some(Code::F3),
            "f4" => key_code = Some(Code::F4),
            "f5" => key_code = Some(Code::F5),
            "f6" => key_code = Some(Code::F6),
            "f7" => key_code = Some(Code::F7),
            "f8" => key_code = Some(Code::F8),
            "f9" => key_code = Some(Code::F9),
            "f10" => key_code = Some(Code::F10),
            "f11" => key_code = Some(Code::F11),
            "f12" => key_code = Some(Code::F12),
            s if s.len() == 1 => {
                let c = s.chars().next().unwrap();
                key_code = Some(char_to_code(c).ok_or_else(|| format!("Unknown hotkey key: {c}"))?);
            }
            other => return Err(format!("Unknown hotkey part: {other}")),
        }
    }

    let code = key_code.ok_or("No key code found in hotkey binding")?;
    Ok((modifiers, code))
}

/// Register a global hotkey from a binding string like "Ctrl+Shift+S".
///
/// Returns the manager (must be kept alive) and the hotkey id.
pub fn register_hotkey(binding: &str) -> Result<(GlobalHotKeyManager, u32), String> {
    let (modifiers, code) = parse_binding(binding)?;

    let manager =
        GlobalHotKeyManager::new().map_err(|e| format!("Failed to create hotkey manager: {e}"))?;

    if modifiers.is_empty() {
        tracing::warn!(
            "Hotkey '{}' has no modifier keys — this will intercept normal typing",
            binding
        );
    }

    let hotkey = HotKey::new(Some(modifiers), code);
    let id = hotkey.id();

    manager
        .register(hotkey)
        .map_err(|e| format!("Failed to register hotkey: {e}"))?;

    Ok((manager, id))
}

fn char_to_code(c: char) -> Option<Code> {
    Some(match c.to_ascii_lowercase() {
        'a' => Code::KeyA,
        'b' => Code::KeyB,
        'c' => Code::KeyC,
        'd' => Code::KeyD,
        'e' => Code::KeyE,
        'f' => Code::KeyF,
        'g' => Code::KeyG,
        'h' => Code::KeyH,
        'i' => Code::KeyI,
        'j' => Code::KeyJ,
        'k' => Code::KeyK,
        'l' => Code::KeyL,
        'm' => Code::KeyM,
        'n' => Code::KeyN,
        'o' => Code::KeyO,
        'p' => Code::KeyP,
        'q' => Code::KeyQ,
        'r' => Code::KeyR,
        's' => Code::KeyS,
        't' => Code::KeyT,
        'u' => Code::KeyU,
        'v' => Code::KeyV,
        'w' => Code::KeyW,
        'x' => Code::KeyX,
        'y' => Code::KeyY,
        'z' => Code::KeyZ,
        '0' => Code::Digit0,
        '1' => Code::Digit1,
        '2' => Code::Digit2,
        '3' => Code::Digit3,
        '4' => Code::Digit4,
        '5' => Code::Digit5,
        '6' => Code::Digit6,
        '7' => Code::Digit7,
        '8' => Code::Digit8,
        '9' => Code::Digit9,
        _ => return None,
    })
}
