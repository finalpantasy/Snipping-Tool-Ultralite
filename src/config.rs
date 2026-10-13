use crate::geometry::Color;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing;

// Per-field serde defaults so a config missing any single key (e.g. hand-edited
// or written by an older version) still parses instead of resetting everything.

fn default_true() -> bool {
    true
}
fn d_color() -> String {
    "red".into()
}
fn d_thickness() -> f32 {
    3.0
}
fn d_theme() -> String {
    "dark".into()
}
fn d_capture() -> String {
    "Ctrl+Shift+S".into()
}

macro_rules! key_default {
    ($fn_name:ident, $key:expr) => {
        fn $fn_name() -> String {
            $key.into()
        }
    };
}
key_default!(k_select, "v");
key_default!(k_arrow, "a");
key_default!(k_rectangle, "r");
key_default!(k_circle, "c");
key_default!(k_rounded_rect, "o");
key_default!(k_line, "l");
key_default!(k_pencil, "p");
key_default!(k_highlight, "h");
key_default!(k_spotlight, "f");
key_default!(k_text, "t");
key_default!(k_pixelate, "b");
key_default!(k_step_marker, "n");
key_default!(k_eyedropper, "i");
key_default!(k_measurement, "m");

#[derive(Debug, Serialize, Deserialize)]
pub struct GeneralConfig {
    #[serde(default = "d_color")]
    pub default_color: String,
    #[serde(default = "d_thickness")]
    pub default_thickness: f32,
    #[serde(default)]
    pub save_directory: String,
    #[serde(default)]
    pub imgur_client_id: String,
    /// When false, captures are not saved to the recent-captures history.
    #[serde(default)]
    pub history_enabled: bool,
    /// UI theme: "dark" or "light".
    #[serde(default = "d_theme")]
    pub theme: String,
    /// Main home/editor X exits the process instead of leaving tray and hotkeys running.
    #[serde(default)]
    pub quit_on_close: bool,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            default_color: d_color(),
            default_thickness: d_thickness(),
            save_directory: String::new(),
            imgur_client_id: String::new(),
            history_enabled: false,
            theme: d_theme(),
            quit_on_close: false,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HotkeyConfig {
    #[serde(default = "d_capture")]
    pub capture: String,
}

impl Default for HotkeyConfig {
    fn default() -> Self {
        Self {
            capture: d_capture(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ShortcutsConfig {
    #[serde(default = "k_select")]
    pub select: String,
    #[serde(default = "k_arrow")]
    pub arrow: String,
    #[serde(default = "k_rectangle")]
    pub rectangle: String,
    #[serde(default = "k_circle")]
    pub circle: String,
    #[serde(default = "k_rounded_rect")]
    pub rounded_rect: String,
    #[serde(default = "k_line")]
    pub line: String,
    #[serde(default = "k_pencil")]
    pub pencil: String,
    #[serde(default = "k_highlight")]
    pub highlight: String,
    #[serde(default = "k_spotlight")]
    pub spotlight: String,
    #[serde(default = "k_text")]
    pub text: String,
    #[serde(default = "k_pixelate")]
    pub pixelate: String,
    #[serde(default = "k_step_marker")]
    pub step_marker: String,
    #[serde(default = "k_eyedropper")]
    pub eyedropper: String,
    #[serde(default = "k_measurement")]
    pub measurement: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ToolbarConfig {
    #[serde(default = "default_true")]
    pub select: bool,
    #[serde(default = "default_true")]
    pub arrow: bool,
    #[serde(default = "default_true")]
    pub rectangle: bool,
    #[serde(default = "default_true")]
    pub circle: bool,
    #[serde(default = "default_true")]
    pub rounded_rect: bool,
    #[serde(default = "default_true")]
    pub line: bool,
    #[serde(default = "default_true")]
    pub pencil: bool,
    #[serde(default = "default_true")]
    pub highlight: bool,
    #[serde(default = "default_true")]
    pub spotlight: bool,
    #[serde(default = "default_true")]
    pub text: bool,
    #[serde(default = "default_true")]
    pub pixelate: bool,
    #[serde(default = "default_true")]
    pub step_marker: bool,
    #[serde(default = "default_true")]
    pub eyedropper: bool,
    #[serde(default = "default_true")]
    pub measurement: bool,
}

impl Default for ToolbarConfig {
    fn default() -> Self {
        Self {
            select: true,
            arrow: true,
            rectangle: true,
            circle: true,
            rounded_rect: true,
            line: true,
            pencil: true,
            highlight: true,
            spotlight: true,
            text: true,
            pixelate: true,
            step_marker: true,
            eyedropper: true,
            measurement: true,
        }
    }
}

impl ToolbarConfig {
    /// Returns the list of visible button indices (0-23) based on which tools are enabled.
    /// Tool buttons 0-13 are conditionally shown; colors 14-18 and actions 19-23 are always shown.
    pub fn visible_button_indices(&self) -> Vec<usize> {
        let mut indices = Vec::new();

        let tool_flags = [
            self.select,
            self.arrow,
            self.rectangle,
            self.circle,
            self.rounded_rect,
            self.line,
            self.pencil,
            self.highlight,
            self.spotlight,
            self.text,
            self.pixelate,
            self.step_marker,
            self.eyedropper,
            self.measurement,
        ];

        for (i, &enabled) in tool_flags.iter().enumerate() {
            if enabled {
                indices.push(i);
            }
        }

        // Colors: always visible (indices 14-18)
        for i in 14..=18 {
            indices.push(i);
        }

        // Actions: always visible (indices 19-23)
        for i in 19..=23 {
            indices.push(i);
        }

        indices
    }

    /// Ordered list of (symbol, label, enabled) for the Settings UI.
    pub fn entries(&self) -> Vec<(&'static str, &'static str, bool)> {
        vec![
            ("->", "Select annotations", self.select),
            (">>", "Arrow", self.arrow),
            ("[]", "Rectangle", self.rectangle),
            ("()", "Circle", self.circle),
            ("[.]", "Rounded Rect", self.rounded_rect),
            ("--", "Line", self.line),
            ("~", "Pencil", self.pencil),
            ("##", "Highlight", self.highlight),
            ("**", "Spotlight", self.spotlight),
            ("Aa", "Text", self.text),
            ("::", "Pixelate", self.pixelate),
            ("1.", "Step Marker", self.step_marker),
            ("/|", "Eyedropper", self.eyedropper),
            ("|<>", "Measurement", self.measurement),
        ]
    }

    /// Toggle a tool by index (0-13).
    pub fn toggle_by_index(&mut self, index: usize) {
        match index {
            0 => self.select = !self.select,
            1 => self.arrow = !self.arrow,
            2 => self.rectangle = !self.rectangle,
            3 => self.circle = !self.circle,
            4 => self.rounded_rect = !self.rounded_rect,
            5 => self.line = !self.line,
            6 => self.pencil = !self.pencil,
            7 => self.highlight = !self.highlight,
            8 => self.spotlight = !self.spotlight,
            9 => self.text = !self.text,
            10 => self.pixelate = !self.pixelate,
            11 => self.step_marker = !self.step_marker,
            12 => self.eyedropper = !self.eyedropper,
            13 => self.measurement = !self.measurement,
            _ => {}
        }
    }
}

impl Default for ShortcutsConfig {
    fn default() -> Self {
        Self {
            select: "v".into(),
            arrow: "a".into(),
            rectangle: "r".into(),
            circle: "c".into(),
            rounded_rect: "o".into(),
            line: "l".into(),
            pencil: "p".into(),
            highlight: "h".into(),
            spotlight: "f".into(),
            text: "t".into(),
            pixelate: "b".into(),
            step_marker: "n".into(),
            eyedropper: "i".into(),
            measurement: "m".into(),
        }
    }
}

impl ShortcutsConfig {
    /// Ordered list of (symbol, label, current key) for UI display.
    pub fn entries(&self) -> Vec<(&'static str, &'static str, &str)> {
        vec![
            ("->", "Select annotations", &self.select),
            (">>", "Arrow", &self.arrow),
            ("[]", "Rectangle", &self.rectangle),
            ("()", "Circle", &self.circle),
            ("[.]", "Rounded Rect", &self.rounded_rect),
            ("--", "Line", &self.line),
            ("~", "Pencil", &self.pencil),
            ("##", "Highlight", &self.highlight),
            ("**", "Spotlight", &self.spotlight),
            ("Aa", "Text", &self.text),
            ("::", "Pixelate", &self.pixelate),
            ("1.", "Step Marker", &self.step_marker),
            ("/|", "Eyedropper", &self.eyedropper),
            ("|<>", "Measurement", &self.measurement),
        ]
    }

    /// Set shortcut by index (0-13).
    pub fn set_by_index(&mut self, index: usize, key: String) {
        match index {
            0 => self.select = key,
            1 => self.arrow = key,
            2 => self.rectangle = key,
            3 => self.circle = key,
            4 => self.rounded_rect = key,
            5 => self.line = key,
            6 => self.pencil = key,
            7 => self.highlight = key,
            8 => self.spotlight = key,
            9 => self.text = key,
            10 => self.pixelate = key,
            11 => self.step_marker = key,
            12 => self.eyedropper = key,
            13 => self.measurement = key,
            _ => {}
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub workflow: WorkflowConfig,
    #[serde(default)]
    pub general: GeneralConfig,
    #[serde(default)]
    pub hotkey: HotkeyConfig,
    #[serde(default)]
    pub shortcuts: ShortcutsConfig,
    #[serde(default)]
    pub toolbar: ToolbarConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowConfig {
    #[serde(default = "default_true")]
    pub open_preview: bool,
    #[serde(default)]
    pub auto_save: bool,
    #[serde(default)]
    pub confirm_upload: bool,
    #[serde(default)]
    pub upload_host: crate::upload::UploadHost,
}

impl Default for WorkflowConfig {
    fn default() -> Self {
        Self {
            open_preview: true,
            auto_save: false,
            confirm_upload: false,
            upload_host: Default::default(),
        }
    }
}

impl Config {
    /// Returns the config file path: `<config_dir>/snipping_tool_ul/config.toml`
    pub fn config_path() -> Option<PathBuf> {
        if let Some(path) = std::env::var_os("SNIPPING_TOOL_UL_CONFIG_DIR") {
            return Some(PathBuf::from(path).join("config.toml"));
        }
        dirs::config_dir().map(|p| p.join("snipping-tool-ul").join("config.toml"))
    }

    /// Load config from disk; creates default if missing; falls back to defaults on parse error.
    pub fn load() -> Self {
        let path = match Self::config_path() {
            Some(p) => p,
            None => {
                tracing::warn!("Could not determine config directory, using defaults");
                return Self::default();
            }
        };

        match std::fs::read_to_string(&path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                tracing::info!("Config file not found, creating with defaults");
                let cfg = Self::default();
                if let Err(e) = cfg.save() {
                    tracing::warn!("Failed to write default config: {e}");
                }
                cfg
            }
            Ok(contents) => match toml::from_str::<Config>(&contents) {
                Ok(cfg) => cfg,
                Err(e) => {
                    // Any later save() would overwrite the user's file with
                    // defaults, so keep their text beside it for repair.
                    // Surface the error visibly; tracing output is invisible
                    // in a windows_subsystem = "windows" build.
                    tracing::warn!("Failed to parse config, using defaults: {e}");
                    let recovery = match std::fs::copy(&path, path.with_extension("toml.bad")) {
                        Ok(_) => "A recovery copy was kept as config.toml.bad.".to_string(),
                        Err(error) => format!("A recovery copy could not be saved: {error}. Copy the original config.toml before saving settings."),
                    };
                    let _ = notify_rust::Notification::new()
                        .summary("Snipping Tool UL")
                        .body(&format!(
                            "config.toml has an error and was ignored: {e}\n{recovery}"
                        ))
                        .timeout(5000)
                        .show();
                    Self::default()
                }
            },
            Err(e) => {
                tracing::warn!("Failed to read config file, using defaults: {e}");
                Self::default()
            }
        }
    }

    /// Write current config to disk as TOML.
    pub fn save(&self) -> Result<(), String> {
        let path = Self::config_path().ok_or("Could not determine config directory")?;
        self.save_to_path(&path)
    }

    fn save_to_path(&self, path: &std::path::Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create config directory: {e}"))?;
        }

        let contents =
            toml::to_string(self).map_err(|e| format!("Failed to serialize config: {e}"))?;

        // Never replace an unparseable config unless its current bytes have
        // first been copied somewhere the user can recover them from.
        match std::fs::read_to_string(path) {
            Ok(existing) if toml::from_str::<Config>(&existing).is_err() => {
                let recovery_path = path.with_extension("toml.bad");
                std::fs::copy(path, &recovery_path).map_err(|e| {
                    format!(
                        "Refusing to overwrite malformed config because its recovery copy could not be saved: {e}"
                    )
                })?;
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => {
                // This can include non-UTF-8 or otherwise unreadable contents.
                // Treat them as unparseable and require a successful copy too.
                let recovery_path = path.with_extension("toml.bad");
                std::fs::copy(path, &recovery_path).map_err(|copy_error| {
                    format!(
                        "Refusing to overwrite unreadable config because its recovery copy could not be saved: {copy_error}"
                    )
                })?;
            }
        }

        // Atomic write: write to a temp file then rename, so a crash mid-write
        // won't corrupt the config.
        let tmp_path = path.with_extension("toml.tmp");
        let write = (|| {
            use std::io::Write;
            let mut file = std::fs::File::create(&tmp_path)?;
            file.write_all(contents.as_bytes())?;
            // Flush to disk before the rename so a power loss cannot leave an
            // empty config.toml, which would silently parse as defaults.
            file.sync_all()
        })();
        if let Err(e) = write {
            let _ = std::fs::remove_file(&tmp_path);
            return Err(format!("Failed to write temp config file: {e}"));
        }
        std::fs::rename(&tmp_path, &path)
            .map_err(|e| format!("Failed to rename config file: {e}"))?;

        Ok(())
    }

    /// Parse the `default_color` string into a `Color`. Accepts the named
    /// Annotation color names or a custom hex value like `#63f2ab`. Falls back to red.
    pub fn default_color(&self) -> Color {
        match self.general.default_color.as_str() {
            "red" => Color::red(),
            "blue" => Color::blue(),
            "green" => Color::green(),
            "yellow" => Color::yellow(),
            "white" => Color::white(),
            "mauve" => Color::mauve(),
            "peach" => Color::peach(),
            "teal" => Color::teal(),
            "sky" => Color::sky(),
            "lavender" => Color::lavender(),
            other => {
                if let Some(c) = parse_hex_color(other) {
                    return c;
                }
                tracing::warn!("Unknown color '{other}', falling back to red");
                Color::red()
            }
        }
    }

    /// Resolve the configured theme into a `ThemeMode` (unknown → dark).
    pub fn theme_mode(&self) -> crate::theme::ThemeMode {
        if self.general.theme.eq_ignore_ascii_case("light") {
            crate::theme::ThemeMode::Light
        } else {
            crate::theme::ThemeMode::Dark
        }
    }

    /// Clamp thickness to the range 1.0..=20.0.
    pub fn clamped_thickness(&self) -> f32 {
        let t = self.general.default_thickness;
        // `nan` is valid TOML and clamp() passes it through unchanged.
        if t.is_finite() {
            t.clamp(1.0, 20.0)
        } else {
            3.0
        }
    }

    /// Resolve auto-save's destination without silently substituting an unavailable path.
    pub fn auto_save_directory(&self) -> Result<PathBuf, String> {
        let chosen = self.general.save_directory.trim();
        if chosen.is_empty() {
            dirs::picture_dir()
                .map(|p| p.join("Snipping Tool UL"))
                .ok_or_else(|| {
                    "Pictures folder unavailable. Choose an auto-save folder in Settings.".into()
                })
        } else {
            let path = PathBuf::from(chosen);
            if !path.is_absolute() {
                return Err(
                    "Auto-save folder must be an absolute path. Change it in Settings.".into(),
                );
            }
            Ok(path)
        }
    }
}

/// Parse a `#rrggbb` hex string into a Color.
pub fn parse_hex_color(s: &str) -> Option<Color> {
    let hex = s.strip_prefix('#')?;
    if hex.len() != 6 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some(Color::new(
        r as f32 / 255.0,
        g as f32 / 255.0,
        b as f32 / 255.0,
        1.0,
    ))
}

#[cfg(test)]
mod config_save_recovery_tests {
    use super::Config;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn failed_recovery_copy_keeps_malformed_config_until_a_copy_succeeds() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "snipping-tool-ul-config-recovery-{}-{unique}",
            std::process::id()
        ));
        std::fs::create_dir_all(&directory).expect("create isolated config directory");
        let path = directory.join("config.toml");
        let recovery_path = path.with_extension("toml.bad");
        let original = b"[general\ndefault_color = \"unfinished\"\n";
        std::fs::write(&path, original).expect("write malformed config");
        std::fs::create_dir(&recovery_path).expect("block recovery destination");

        let config = Config::default();
        let error = config
            .save_to_path(&path)
            .expect_err("save must stop when recovery copy fails");
        assert!(error.contains("recovery copy could not be saved"));
        assert_eq!(
            std::fs::read(&path).expect("read original config"),
            original
        );

        std::fs::remove_dir(&recovery_path).expect("unblock recovery destination");
        config
            .save_to_path(&path)
            .expect("save after recovery copy can be made");
        assert_eq!(
            std::fs::read(&recovery_path).expect("read recovery copy"),
            original
        );
        let saved = std::fs::read_to_string(&path).expect("read replacement config");
        toml::from_str::<Config>(&saved).expect("replacement config is valid TOML");

        std::fs::remove_dir_all(directory).expect("remove isolated config directory");
    }
}
