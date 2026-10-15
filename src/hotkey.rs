use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use global_hotkey::GlobalHotKeyManager;

/// Builds a binding from a key press in the Settings recorder, or `None`
/// for keys that cannot be a global hotkey or when no Ctrl/Alt/Super
/// modifier is held. It reads the physical key, as the global hotkey does:
/// the typed character changes with Shift and layout, so Ctrl+Shift+1
/// arrives as "!" and AZERTY's unshifted top row has no digits.
pub fn binding_from_key(
    mods: winit::keyboard::ModifiersState,
    key: &winit::keyboard::PhysicalKey,
) -> Option<String> {
    use winit::keyboard::{KeyCode, PhysicalKey};
    let PhysicalKey::Code(code) = key else {
        return None;
    };
    let name = format!("{code:?}");
    let key_part = if let Some(c) = name.strip_prefix("Key").filter(|c| c.len() == 1) {
        c.to_string()
    } else if let Some(d) = name.strip_prefix("Digit").filter(|d| d.len() == 1) {
        d.to_string()
    } else if *code == KeyCode::PrintScreen
        || name
            .strip_prefix('F')
            .and_then(|n| n.parse::<u8>().ok())
            .is_some_and(|n| (1..=12).contains(&n))
    {
        name
    } else {
        return None;
    };
    // Require a strong modifier so the hotkey can't swallow normal typing.
    if !(mods.control_key() || mods.alt_key() || mods.super_key()) {
        return None;
    }
    let mut parts: Vec<String> = Vec::new();
    for (held, label) in [
        (mods.control_key(), "Ctrl"),
        (mods.shift_key(), "Shift"),
        (mods.alt_key(), "Alt"),
        (mods.super_key(), "Super"),
    ] {
        if held {
            parts.push(label.into());
        }
    }
    parts.push(key_part);
    let binding = parts.join("+");
    // Final validation through the real parser.
    parse_binding(&binding).ok()?;
    Some(binding)
}

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
    // A bare letter or digit would swallow ordinary typing system-wide;
    // function keys and PrintScreen are the only keys allowed without modifiers.
    let mut needs_modifier = false;

    let mut set_key = |code: Code, bare_ok: bool| -> Result<(), String> {
        if key_code.is_some() {
            return Err("A hotkey can have only one non-modifier key".into());
        }
        key_code = Some(code);
        needs_modifier = !bare_ok;
        Ok(())
    };

    for part in &parts {
        match part.trim().to_lowercase().as_str() {
            "ctrl" | "control" => modifiers |= Modifiers::CONTROL,
            "shift" => modifiers |= Modifiers::SHIFT,
            "alt" => modifiers |= Modifiers::ALT,
            "super" | "meta" | "win" => modifiers |= Modifiers::SUPER,
            "printscreen" => set_key(Code::PrintScreen, true)?,
            "f1" => set_key(Code::F1, true)?,
            "f2" => set_key(Code::F2, true)?,
            "f3" => set_key(Code::F3, true)?,
            "f4" => set_key(Code::F4, true)?,
            "f5" => set_key(Code::F5, true)?,
            "f6" => set_key(Code::F6, true)?,
            "f7" => set_key(Code::F7, true)?,
            "f8" => set_key(Code::F8, true)?,
            "f9" => set_key(Code::F9, true)?,
            "f10" => set_key(Code::F10, true)?,
            "f11" => set_key(Code::F11, true)?,
            "f12" => set_key(Code::F12, true)?,
            s if s.len() == 1 => {
                let c = s.chars().next().unwrap();
                let code = char_to_code(c).ok_or_else(|| format!("Unknown hotkey key: {c}"))?;
                set_key(code, false)?;
            }
            other => return Err(format!("Unknown hotkey part: {other}")),
        }
    }

    let code = key_code.ok_or("No key code found in hotkey binding")?;
    if needs_modifier && modifiers.is_empty() {
        return Err("Add Ctrl, Alt, Shift or Win: a bare key would block normal typing".into());
    }
    Ok((modifiers, code))
}

/// The shortcut that snips the previous area again, derived from the capture
/// shortcut so there is nothing extra to set up: Alt is added (Ctrl+Shift+S
/// becomes Ctrl+Shift+Alt+S). A bare PrintScreen gets Shift instead, because
/// Alt+PrintScreen is Windows' own window screenshot. `None` when the capture
/// shortcut already uses Alt and Shift.
pub fn repeat_binding(capture: &str) -> Option<String> {
    let (mods, code) = parse_binding(capture).ok()?;
    // A bare F-key would become Alt+F4 and similar; leave those alone.
    if mods.is_empty() && code != Code::PrintScreen {
        return None;
    }
    let mut mods = mods;
    if code == Code::PrintScreen && mods.is_empty() {
        mods |= Modifiers::SHIFT;
    } else if !mods.contains(Modifiers::ALT) {
        mods |= Modifiers::ALT;
    } else if !mods.contains(Modifiers::SHIFT) {
        mods |= Modifiers::SHIFT;
    } else {
        return None;
    }
    // Windows owns Alt+F4 (close window), Alt+PrintScreen (window shot) and
    // Shift+Alt+PrintScreen (High Contrast).
    if (mods == Modifiers::ALT && code == Code::F4)
        || (code == Code::PrintScreen && mods.contains(Modifiers::ALT))
    {
        return None;
    }
    // The key may be written anywhere ("S+Ctrl+Shift"); it is the one part
    // that is not a modifier name.
    let key = capture.split('+').map(str::trim).find(|part| {
        !matches!(
            part.to_lowercase().as_str(),
            "ctrl" | "control" | "shift" | "alt" | "super" | "meta" | "win"
        )
    })?;
    let mut parts: Vec<&str> = Vec::new();
    for (flag, label) in [
        (Modifiers::CONTROL, "Ctrl"),
        (Modifiers::SHIFT, "Shift"),
        (Modifiers::ALT, "Alt"),
        (Modifiers::SUPER, "Super"),
    ] {
        if mods.contains(flag) {
            parts.push(label);
        }
    }
    parts.push(key);
    Some(parts.join("+"))
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
