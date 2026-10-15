use tray_icon::menu::{CheckMenuItem, Menu, MenuItem};
use tray_icon::{Icon, TrayIconBuilder};

pub struct TrayState {
    pub capture_id: tray_icon::menu::MenuId,
    capture_item: MenuItem,
    /// Snips the previous area again; disabled until its shortcut exists.
    pub repeat_id: tray_icon::menu::MenuId,
    repeat_item: MenuItem,
    pub window_capture_id: tray_icon::menu::MenuId,
    pub delay_3_id: tray_icon::menu::MenuId,
    pub delay_5_id: tray_icon::menu::MenuId,
    pub delay_10_id: tray_icon::menu::MenuId,
    pub history_id: tray_icon::menu::MenuId,
    pub autostart_id: tray_icon::menu::MenuId,
    pub autostart_check: tray_icon::menu::CheckMenuItem,
    pub settings_id: tray_icon::menu::MenuId,
    pub about_id: tray_icon::menu::MenuId,
    pub quit_id: tray_icon::menu::MenuId,
    /// Set once the weekly check finds a newer release.
    pub update_id: Option<tray_icon::menu::MenuId>,
    menu: Menu,
    pub _tray: tray_icon::TrayIcon,
}

pub fn create_tray() -> Result<TrayState, String> {
    // Prepared pixels: no runtime SVG parsing, PNG decoding or icon rasterization.
    let icon = Icon::from_rgba(tray_rgba(system_taskbar_is_light()).to_vec(), 32, 32)
        .map_err(|e| e.to_string())?;

    // Create menu with Capture, About, Quit
    let menu = Menu::new();
    let capture_item = MenuItem::new("Capture screenshot", true, None);
    let repeat_item = MenuItem::new("Snip same area again", true, None);
    let window_capture_item = MenuItem::new("Capture Window", true, None);
    let delay_3_item = MenuItem::new("Capture in 3s", true, None);
    let delay_5_item = MenuItem::new("Capture in 5s", true, None);
    let delay_10_item = MenuItem::new("Capture in 10s", true, None);
    let history_item = MenuItem::new("History", true, None);
    let autostart_item =
        CheckMenuItem::new("Start on login", true, crate::autostart::is_enabled(), None);
    let settings_item = MenuItem::new("Settings", true, None);
    let about_item = MenuItem::new("About", true, None);
    let quit_item = MenuItem::new("Quit", true, None);
    let capture_id = capture_item.id().clone();
    let repeat_id = repeat_item.id().clone();
    let window_capture_id = window_capture_item.id().clone();
    let delay_3_id = delay_3_item.id().clone();
    let delay_5_id = delay_5_item.id().clone();
    let delay_10_id = delay_10_item.id().clone();
    let history_id = history_item.id().clone();
    let autostart_id = autostart_item.id().clone();
    let settings_id = settings_item.id().clone();
    let about_id = about_item.id().clone();
    let quit_id = quit_item.id().clone();
    menu.append(&capture_item).map_err(|e| e.to_string())?;
    menu.append(&repeat_item).map_err(|e| e.to_string())?;
    menu.append(&window_capture_item)
        .map_err(|e| e.to_string())?;
    menu.append(&delay_3_item).map_err(|e| e.to_string())?;
    menu.append(&delay_5_item).map_err(|e| e.to_string())?;
    menu.append(&delay_10_item).map_err(|e| e.to_string())?;
    menu.append(&history_item).map_err(|e| e.to_string())?;
    menu.append(&autostart_item).map_err(|e| e.to_string())?;
    menu.append(&settings_item).map_err(|e| e.to_string())?;
    menu.append(&about_item).map_err(|e| e.to_string())?;
    menu.append(&quit_item).map_err(|e| e.to_string())?;

    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu.clone()))
        .with_menu_on_left_click(false)
        .with_tooltip("Snipping Tool UL")
        .with_icon(icon)
        .build()
        .map_err(|e| e.to_string())?;

    Ok(TrayState {
        capture_id,
        capture_item,
        repeat_id,
        repeat_item,
        window_capture_id,
        delay_3_id,
        delay_5_id,
        delay_10_id,
        history_id,
        autostart_id,
        autostart_check: autostart_item,
        settings_id,
        about_id,
        quit_id,
        update_id: None,
        menu,
        _tray: tray,
    })
}

impl TrayState {
    /// Adds an "Update available" item at the top of the menu, once.
    pub fn show_update(&mut self, version: &str) -> Result<(), String> {
        if self.update_id.is_some() {
            return Ok(());
        }
        let item = MenuItem::new(format!("Update available: {version}"), true, None);
        self.menu.prepend(&item).map_err(|e| e.to_string())?;
        self.update_id = Some(item.id().clone());
        Ok(())
    }

    pub fn update_repeat_hotkey(&self, hotkey: Option<&str>) {
        self.repeat_item.set_text(match hotkey {
            Some(binding) => format!("Snip same area again\t{binding}"),
            None => "Snip same area again".into(),
        });
    }

    pub fn update_capture_hotkey(&self, hotkey: Option<&str>) {
        let (label, tooltip) = capture_hotkey_text(hotkey);
        self.capture_item.set_text(label);
        if let Err(error) = self._tray.set_tooltip(Some(tooltip)) {
            tracing::warn!("Could not update capture shortcut tooltip: {error}");
        }
    }
}

fn capture_hotkey_text(hotkey: Option<&str>) -> (String, String) {
    match hotkey {
        Some(binding) => (
            format!("Capture screenshot\t{binding}"),
            format!("Snipping Tool UL\nCapture screenshot: {binding}"),
        ),
        None => (
            "Capture screenshot (hotkey unavailable)".into(),
            "Snipping Tool UL\nCapture hotkey unavailable. Use New or Settings.".into(),
        ),
    }
}

pub fn tray_rgba(light_taskbar: bool) -> &'static [u8; 32 * 32 * 4] {
    if light_taskbar {
        include_bytes!("../assets/ul-tray-on-light.rgba")
    } else {
        include_bytes!("../assets/ul-tray-on-dark.rgba")
    }
}

fn system_taskbar_is_light() -> bool {
    #[cfg(target_os = "windows")]
    unsafe {
        use windows::Win32::System::Registry::*;
        let mut light = 0u32;
        let mut bytes = std::mem::size_of::<u32>() as u32;
        let _ = RegGetValueW(
            HKEY_CURRENT_USER,
            windows::core::w!("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize"),
            windows::core::w!("SystemUsesLightTheme"),
            RRF_RT_REG_DWORD,
            None,
            Some(&mut light as *mut _ as _),
            Some(&mut bytes),
        );
        light == 1
    }
    #[cfg(not(target_os = "windows"))]
    {
        false
    }
}
