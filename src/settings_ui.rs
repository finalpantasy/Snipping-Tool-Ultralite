use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::sync::Arc;

use tiny_skia::Pixmap;
use winit::window::Window;

use crate::config::Config;
use crate::geometry::{Color, Point};
use crate::icons::IconCache;
use crate::theme;
use crate::ui::{measure, text_bounds, Canvas, Rect};

/// Width / height of the settings window in logical pixels.
pub const WIN_W: u32 = 420;
pub const WIN_H: u32 = 560;

/// Same annotation colors used by the editor, with stable saved names.
const COLOR_CHOICES: &[(&str, Color)] = &[
    ("red", Color::red()),
    ("blue", Color::blue()),
    ("green", Color::green()),
    ("yellow", Color::yellow()),
    ("mauve", Color::mauve()),
];

const TABS: [&str; 4] = ["General", "Shortcuts", "Tools", "Windows"];

// Layout grid, logical pixels. Sections are cards; rows put the label on the
// left and the control flush right, so every control shares one edge.
const LEFT: f32 = 24.;
const CONTENT_W: f32 = WIN_W as f32 - LEFT * 2.;
const CARD_X: f32 = 16.;
const CARD_W: f32 = WIN_W as f32 - CARD_X * 2.;
const ROW_X: f32 = 30.;
const ROW_R: f32 = WIN_W as f32 - 30.;
const ROW_H: f32 = 30.;
const LIST_ROW_H: f32 = 24.;
const LIST_TOP: f32 = 121.;

/// Actions produced by clicking a region.
#[derive(Debug, Clone, Copy)]
pub enum Action {
    SelectColor(usize),
    ThicknessDown,
    ThicknessUp,
    BrowseDir,
    ToggleAutostart,
    ToggleHistory,
    ToggleTheme,
    SetCloseBehavior(bool),
    TakeOverWindows,
    RestoreWindows,
    ClickShortcut(usize),
    ClickHotkey,
    ToggleToolbar(usize),
    SaveClose,
    SwitchTab(usize),
}

/// A clickable rectangle mapped to an action.
struct HitRect {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    action: Action,
}

pub struct SettingsWindow {
    pub window: Arc<Window>,
    pub surface: crate::native_surface::Surface,
    ui: SettingsUi,
    frame: Option<Pixmap>,
    /// Theme the native caption was last styled for; restyle only on change.
    styled: Option<theme::ThemeMode>,
}
impl std::ops::Deref for SettingsWindow {
    type Target = SettingsUi;
    fn deref(&self) -> &Self::Target {
        &self.ui
    }
}
impl std::ops::DerefMut for SettingsWindow {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.ui
    }
}
pub struct SettingsUi {
    pub config: Config,
    icons: IconCache,
    pub needs_redraw: bool,
    hit_rects: Vec<HitRect>,
    pub cursor_pos: (f32, f32),
    /// Index of the shortcut currently being edited (0-13), or None.
    pub editing_shortcut: Option<usize>,
    /// True while waiting for the user to press the new global-hotkey combo.
    pub editing_hotkey: bool,
    /// Active tab index: 0=General, 1=Shortcuts, 2=Tools, 3=Windows.
    pub active_tab: usize,
    pub integration_pending: bool,
    pub integration_status: String,
    /// Index of the currently hovered hit rect (for redraw optimization).
    hovered: Option<usize>,
    /// Pending result channel for the async folder-picker dialog.
    browse_rx: Option<Receiver<Option<String>>>,
    /// Auto-start choice: read from the registry once, written only on Save.
    autostart: Option<bool>,
}

/// Paints controls and records their logical hit targets in one pass.
struct Painter<'a> {
    c: Canvas<'a>,
    hits: &'a mut Vec<HitRect>,
    cursor: (f32, f32),
}

impl Painter<'_> {
    fn hover(&self, r: Rect) -> bool {
        let (x, y) = self.cursor;
        x >= r.x && x <= r.x + r.w && y >= r.y && y <= r.y + r.h
    }

    fn hit(&mut self, r: Rect, action: Action) {
        self.hits.push(HitRect {
            x: r.x,
            y: r.y,
            w: r.w,
            h: r.h,
            action,
        });
    }

    fn width(&self, text: &str, size: f32) -> f32 {
        measure(text, size * self.c.scale) / self.c.scale
    }

    /// Draw text whose cap height is centered on `mid`.
    fn text_mid(&mut self, x: f32, mid: f32, text: &str, size: f32, rgb: (u8, u8, u8)) {
        let (_, y0, _, y1) = text_bounds("H", size * self.c.scale);
        let top = mid - (y0 + y1) / 2. / self.c.scale;
        self.c.label(x, top, text, size, rgb);
    }

    fn text_right(&mut self, right: f32, mid: f32, text: &str, size: f32, rgb: (u8, u8, u8)) {
        let w = self.width(text, size);
        self.text_mid(right - w, mid, text, size, rgb);
    }

    fn header(&mut self) {
        self.c.icon("gear", LEFT, 18., 22., theme::text_normal());
        self.c
            .label(LEFT + 32., 20., "Settings", 16., theme::text_normal());
    }

    fn tabs(&mut self, active: usize) {
        let track = Rect::new(LEFT, 50., CONTENT_W, 32.);
        self.c.round(track, 9., theme::bg_2(), 0.7);
        let w = CONTENT_W / TABS.len() as f32;
        for (i, name) in TABS.iter().enumerate() {
            let r = Rect::new(LEFT + i as f32 * w, track.y, w, track.h);
            let pill = Rect::new(r.x + 3., r.y + 3., r.w - 6., r.h - 6.);
            if i == active {
                self.c.round(pill, 7., theme::bg_4(), 1.);
            } else if self.hover(r) {
                self.c.round(pill, 7., theme::bg_3(), 0.8);
            }
            let color = if i == active {
                theme::text_normal()
            } else {
                theme::text_muted()
            };
            self.c.centered_in(r, name, 13., color);
            self.hit(r, Action::SwitchTab(i));
        }
    }

    /// Section title plus its card; returns the top of the first row.
    fn section(&mut self, y: f32, title: &str, rows_h: f32) -> f32 {
        self.c.label(LEFT, y, title, 12., theme::text_muted());
        self.c.round(
            Rect::new(CARD_X, y + 18., CARD_W, rows_h + 6.),
            10.,
            theme::bg_2(),
            0.55,
        );
        y + 21.
    }

    fn divider(&mut self, y: f32) {
        self.c.round(
            Rect::new(ROW_X, y, ROW_R - ROW_X, 1.),
            0.,
            theme::border(),
            0.4,
        );
    }

    fn row(&mut self, top: f32, h: f32, label: &str) -> Rect {
        self.text_mid(ROW_X, top + h / 2., label, 13., theme::text_normal());
        Rect::new(CARD_X, top, CARD_W, h)
    }

    fn switch(&mut self, row: Rect, on: bool, action: Action) {
        let mid = row.y + row.h / 2.;
        let track = Rect::new(ROW_R - 40., mid - 11., 40., 22.);
        let hovered = self.hover(row);
        if on {
            self.c
                .round(track, 11., theme::accent(), if hovered { 0.85 } else { 1. });
        } else {
            self.c
                .round(track, 11., theme::bg_4(), if hovered { 0.95 } else { 0.6 });
            self.c.outline(track, 11., theme::border_strong(), 1.);
        }
        let k = 14.;
        let kx = if on {
            track.x + track.w - 4. - k
        } else {
            track.x + 4.
        };
        let knob = if on {
            theme::text_on_accent()
        } else {
            theme::text_soft()
        };
        self.c
            .round(Rect::new(kx, mid - k / 2., k, k), k / 2., knob, 1.);
        self.hit(row, action);
    }

    /// Right-aligned segmented control; returns each segment's rect.
    fn segmented(&mut self, row: Rect, options: &[&str], selected: usize) -> Vec<Rect> {
        let mid = row.y + row.h / 2.;
        let widths: Vec<f32> = options.iter().map(|o| self.width(o, 12.) + 22.).collect();
        let total: f32 = widths.iter().sum::<f32>() + 4.;
        let track = Rect::new(ROW_R - total, mid - 13., total, 26.);
        self.c.round(track, 7., theme::bg_3(), 0.55);
        let mut x = track.x + 2.;
        let mut rects = Vec::with_capacity(options.len());
        for (i, (label, w)) in options.iter().zip(&widths).enumerate() {
            let seg = Rect::new(x, track.y + 2., *w, track.h - 4.);
            let color = if i == selected {
                self.c.round(seg, 5., theme::accent(), 1.);
                theme::text_on_accent()
            } else {
                if self.hover(seg) {
                    self.c.round(seg, 5., theme::bg_4(), 0.8);
                }
                theme::text_normal()
            };
            self.c.centered_in(seg, label, 12., color);
            rects.push(seg);
            x += w;
        }
        rects
    }

    fn small_button(&mut self, r: Rect) {
        let alpha = if self.hover(r) { 0.95 } else { 0.65 };
        self.c.round(r, 6., theme::bg_3(), alpha);
    }

    fn stepper(&mut self, row: Rect, value: &str) {
        let mid = row.y + row.h / 2.;
        let b = 24.;
        let plus = Rect::new(ROW_R - b, mid - b / 2., b, b);
        let minus = Rect::new(plus.x - 44. - b, plus.y, b, b);
        let ink = theme::text_normal();
        for (r, action) in [(minus, Action::ThicknessDown), (plus, Action::ThicknessUp)] {
            self.small_button(r);
            let (cx, cy) = (r.x + r.w / 2., r.y + r.h / 2.);
            self.c
                .round(Rect::new(cx - 5., cy - 0.75, 10., 1.5), 0., ink, 1.);
            if matches!(action, Action::ThicknessUp) {
                self.c
                    .round(Rect::new(cx - 0.75, cy - 5., 1.5, 10.), 0., ink, 1.);
            }
            self.hit(r, action);
        }
        self.c
            .centered_in(Rect::new(minus.x + b, plus.y, 44., b), value, 13., ink);
    }

    fn swatches(&mut self, row: Rect, selected: &str) {
        let mid = row.y + row.h / 2.;
        let d = 20.;
        let gap = 10.;
        let n = COLOR_CHOICES.len() as f32;
        let mut x = ROW_R - n * d - (n - 1.) * gap;
        for (i, &(name, color)) in COLOR_CHOICES.iter().enumerate() {
            let dot = Rect::new(x, mid - d / 2., d, d);
            let ring = Rect::new(dot.x - 3., dot.y - 3., d + 6., d + 6.);
            if selected == name {
                self.c.outline(ring, ring.w / 2., theme::accent_alt(), 2.);
            } else if self.hover(ring) {
                self.c
                    .outline(ring, ring.w / 2., theme::border_strong(), 1.);
            }
            let rgb = |v: f32| (v * 255.).round() as u8;
            self.c
                .round(dot, d / 2., (rgb(color.r), rgb(color.g), rgb(color.b)), 1.);
            self.hit(ring, Action::SelectColor(i));
            x += d + gap;
        }
    }

    /// Keycap-style chip used for the hotkey and tool shortcuts.
    fn key_chip(&mut self, r: Rect, text: &str, editing: bool, hovered: bool, rgb: (u8, u8, u8)) {
        self.c.round(
            r,
            6.,
            theme::bg_3(),
            if hovered || editing { 0.95 } else { 0.7 },
        );
        if editing {
            self.c.outline(r, 6., theme::accent_alt(), 1.5);
        }
        self.c.centered_in(r, text, 12., rgb);
    }

    fn list_card(&mut self, intro: &str, rows: usize) {
        self.c.label(LEFT, 96., intro, 12., theme::text_muted());
        self.c.round(
            Rect::new(CARD_X, LIST_TOP - 3., CARD_W, rows as f32 * LIST_ROW_H + 6.),
            10.,
            theme::bg_2(),
            0.55,
        );
    }

    fn list_row(&mut self, i: usize, label: &str) -> Rect {
        let row = Rect::new(CARD_X, LIST_TOP + i as f32 * LIST_ROW_H, CARD_W, LIST_ROW_H);
        if self.hover(row) {
            self.c.round(
                Rect::new(row.x + 4., row.y + 1., row.w - 8., row.h - 2.),
                6.,
                theme::bg_3(),
                0.6,
            );
        }
        let mid = row.y + row.h / 2.;
        if let Some(icon) = crate::overlay::toolbar::BUTTONS.get(i).and_then(|b| b.icon) {
            self.c
                .icon(icon, ROW_X, mid - 8., 16., theme::text_normal());
        }
        self.text_mid(ROW_X + 28., mid, label, 13., theme::text_normal());
        row
    }

    fn footer(&mut self) {
        let h = WIN_H as f32;
        self.c.round(
            Rect::new(LEFT, h - 72., CONTENT_W, 1.),
            0.,
            theme::border(),
            0.5,
        );
        self.text_mid(
            LEFT,
            h - 37.,
            "Esc or close discards changes",
            11.,
            theme::text_muted(),
        );
        let save = Rect::new(WIN_W as f32 - LEFT - 132., h - 54., 132., 34.);
        let hovered = self.hover(save);
        if hovered {
            self.c.outline(
                Rect::new(save.x - 2., save.y - 2., save.w + 4., save.h + 4.),
                9.,
                theme::accent_alt(),
                1.5,
            );
        }
        self.c
            .button(save, "Save & Close", None, true, hovered, true);
        self.hit(save, Action::SaveClose);
    }
}

/// Shorten from the left with an ellipsis so the end of a path stays visible.
fn fit_left(text: &str, max_w: f32, size: f32, scale: f32) -> String {
    let px = size * scale;
    if measure(text, px) / scale <= max_w {
        return text.to_string();
    }
    let budget = (max_w - measure("...", px) / scale) * scale;
    let mut used = 0.;
    let mut start = text.len();
    for (i, ch) in text.char_indices().rev() {
        used += crate::ui::font().metrics(ch, px).advance_width;
        if used > budget {
            break;
        }
        start = i;
    }
    format!("...{}", &text[start..])
}

impl SettingsUi {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            icons: IconCache::new(),
            needs_redraw: true,
            hit_rects: Vec::new(),
            cursor_pos: (0.0, 0.0),
            editing_shortcut: None,
            editing_hotkey: false,
            hovered: None,
            active_tab: 0,
            integration_pending: false,
            integration_status: String::new(),
            browse_rx: None,
            autostart: None,
        }
    }

    /// True while a folder-picker dialog is open on a background thread.
    pub fn browse_pending(&self) -> bool {
        self.browse_rx.is_some()
    }

    /// Poll the async folder picker. Returns true when the save directory
    /// was just updated (caller should redraw).
    pub fn poll_browse(&mut self) -> bool {
        let Some(rx) = &self.browse_rx else {
            return false;
        };
        match rx.try_recv() {
            Ok(Some(dir)) => {
                self.config.general.save_directory = dir;
                self.browse_rx = None;
                self.needs_redraw = true;
                true
            }
            Ok(None) => {
                self.browse_rx = None;
                false
            }
            Err(TryRecvError::Empty) => false,
            Err(TryRecvError::Disconnected) => {
                self.browse_rx = None;
                false
            }
        }
    }

    /// Full render of the settings UI into the softbuffer surface.
    /// Render directly at physical resolution. Hit targets remain logical.
    pub fn draw(&mut self, pixmap: &mut Pixmap) {
        crate::ui::glass_fill(pixmap);
        let scale = pixmap.width() as f32 / WIN_W as f32;
        let autostart = *self
            .autostart
            .get_or_insert_with(crate::autostart::is_enabled);
        let mut hits = std::mem::take(&mut self.hit_rects);
        hits.clear();
        let mut p = Painter {
            c: Canvas {
                pixmap,
                scale,
                icons: &mut self.icons,
            },
            hits: &mut hits,
            cursor: self.cursor_pos,
        };
        p.header();
        p.tabs(self.active_tab);
        match self.active_tab {
            0 => render_general_tab(&mut p, &self.config, autostart, self.editing_hotkey),
            1 => render_shortcuts_tab(&mut p, &self.config, self.editing_shortcut),
            2 => render_toolbar_tab(&mut p, &self.config),
            _ => {
                crate::windows_integration::draw_cached(
                    p.c.pixmap,
                    self.integration_pending,
                    &self.integration_status,
                    Some(Point::new(self.cursor_pos.0, self.cursor_pos.1)),
                    p.c.icons,
                );
                if !self.integration_pending {
                    p.hit(
                        crate::windows_integration::TAKEOVER_BUTTON,
                        Action::TakeOverWindows,
                    );
                    p.hit(
                        crate::windows_integration::RESTORE_BUTTON,
                        Action::RestoreWindows,
                    );
                }
            }
        }
        p.footer();
        self.hit_rects = hits;
        self.needs_redraw = false;
    }

    /// Update cursor position and return whether a redraw is needed.
    pub fn on_logical_cursor_moved(&mut self, x: f32, y: f32) -> bool {
        self.cursor_pos = (x, y);
        // Only redraw when the hovered element actually changes
        let old_hovered = self.hovered;
        self.hovered = self
            .hit_rects
            .iter()
            .position(|hr| x >= hr.x && x <= hr.x + hr.w && y >= hr.y && y <= hr.y + hr.h);
        old_hovered != self.hovered
    }

    /// Hit-test a click and return the action, if any.
    pub fn on_click(&self, x: f32, y: f32) -> Option<Action> {
        for hr in &self.hit_rects {
            if x >= hr.x && x <= hr.x + hr.w && y >= hr.y && y <= hr.y + hr.h {
                return Some(hr.action);
            }
        }
        None
    }

    /// Handle an action, returning true if the window should close.
    pub fn handle_action(&mut self, action: Action) -> bool {
        match action {
            Action::SetCloseBehavior(quit) => {
                self.config.general.quit_on_close = quit;
                self.needs_redraw = true;
                false
            }
            Action::SelectColor(idx) => {
                if let Some(&(name, _)) = COLOR_CHOICES.get(idx) {
                    self.config.general.default_color = name.to_string();
                    self.needs_redraw = true;
                }
                false
            }
            Action::ThicknessDown => {
                self.config.general.default_thickness =
                    (self.config.general.default_thickness - 0.5).max(1.0);
                self.needs_redraw = true;
                false
            }
            Action::ThicknessUp => {
                self.config.general.default_thickness =
                    (self.config.general.default_thickness + 0.5).min(20.0);
                self.needs_redraw = true;
                false
            }
            Action::BrowseDir => {
                // Run the dialog on a background thread so the event loop
                // (tray, hotkey, pinned windows) stays responsive; the result
                // is applied via poll_browse() from about_to_wait.
                if self.browse_rx.is_none() {
                    let (tx, rx) = channel();
                    self.browse_rx = Some(rx);
                    std::thread::spawn(move || {
                        let result = rfd::FileDialog::new()
                            .pick_folder()
                            .map(|p| p.to_string_lossy().to_string());
                        let _ = tx.send(result);
                    });
                }
                false
            }
            Action::ToggleHistory => {
                self.config.general.history_enabled = !self.config.general.history_enabled;
                self.needs_redraw = true;
                false
            }
            Action::TakeOverWindows | Action::RestoreWindows => false,
            Action::ToggleTheme => {
                let now_light = !self.config.general.theme.eq_ignore_ascii_case("light");
                self.config.general.theme = if now_light { "light" } else { "dark" }.to_string();
                theme::set_mode(self.config.theme_mode());
                self.needs_redraw = true;
                false
            }
            Action::ClickHotkey => {
                self.editing_hotkey = true;
                self.editing_shortcut = None;
                self.needs_redraw = true;
                false
            }
            Action::ToggleAutostart => {
                // Applied on Save like every other setting, so Esc/close discards it.
                let current = self.autostart.unwrap_or_else(crate::autostart::is_enabled);
                self.autostart = Some(!current);
                self.needs_redraw = true;
                false
            }
            Action::ClickShortcut(idx) => {
                self.editing_shortcut = Some(idx);
                self.editing_hotkey = false;
                self.needs_redraw = true;
                false
            }
            Action::ToggleToolbar(idx) => {
                self.config.toolbar.toggle_by_index(idx);
                self.needs_redraw = true;
                false
            }
            Action::SaveClose => {
                if let Err(e) = self.config.save() {
                    tracing::error!("Failed to save config: {e}");
                }
                if let Some(want) = self.autostart {
                    if want != crate::autostart::is_enabled() {
                        if let Err(e) = crate::autostart::set_enabled(want) {
                            tracing::error!("Auto-start toggle failed: {e}");
                        }
                    }
                }
                true
            }
            Action::SwitchTab(idx) => {
                self.active_tab = idx;
                self.editing_shortcut = None;
                self.editing_hotkey = false;
                self.needs_redraw = true;
                false
            }
        }
    }

    /// Handle a key press while a shortcut is being edited.
    /// Returns `true` if the key was consumed (a shortcut was being edited).
    pub fn on_key_press(&mut self, key: &str) -> bool {
        if let Some(idx) = self.editing_shortcut {
            if !key.is_empty() {
                self.config.shortcuts.set_by_index(idx, key.to_string());
            }
            self.editing_shortcut = None;
            self.needs_redraw = true;
            true
        } else {
            false
        }
    }
}

fn render_general_tab(p: &mut Painter, config: &Config, autostart: bool, editing_hotkey: bool) {
    let general = &config.general;

    // Annotation defaults
    let top = p.section(96., "Annotation", ROW_H * 2.);
    let row = p.row(top, ROW_H, "Default color");
    p.swatches(row, &general.default_color);
    p.divider(top + ROW_H);
    let row = p.row(top + ROW_H, ROW_H, "Line thickness");
    p.stepper(row, &format!("{:.1}", general.default_thickness));

    // Capture and output
    let folder_h = 44.;
    let top = p.section(188., "Capture", ROW_H * 2. + folder_h);
    let row = p.row(top, ROW_H, "Capture hotkey");
    let hotkey = if editing_hotkey {
        "Press keys..."
    } else {
        config.hotkey.capture.as_str()
    };
    let chip_w = (p.width(hotkey, 12.) + 24.).max(110.);
    let mid = row.y + row.h / 2.;
    let hovered = p.hover(row);
    p.key_chip(
        Rect::new(ROW_R - chip_w, mid - 12., chip_w, 24.),
        hotkey,
        editing_hotkey,
        hovered,
        theme::text_normal(),
    );
    p.hit(row, Action::ClickHotkey);
    p.divider(top + ROW_H);

    let ftop = top + ROW_H;
    p.text_mid(ROW_X, ftop + 13., "Save folder", 13., theme::text_normal());
    let change = Rect::new(ROW_R - 76., ftop + folder_h / 2. - 14., 76., 28.);
    let dir = if general.save_directory.is_empty() {
        "Pictures\\Snipping Tool UL"
    } else {
        general.save_directory.as_str()
    };
    let dir = fit_left(dir, change.x - 12. - ROW_X, 11.5, p.c.scale);
    p.text_mid(ROW_X, ftop + 31., &dir, 11.5, theme::text_muted());
    let folder_row = Rect::new(CARD_X, ftop, CARD_W, folder_h);
    let hovered = p.hover(folder_row);
    p.c.button(change, "Change", None, false, hovered, true);
    p.hit(folder_row, Action::BrowseDir);
    p.divider(ftop + folder_h);

    let row = p.row(ftop + folder_h, ROW_H, "Uploads");
    let uploads = "No setup · Options > Image host";
    p.text_right(ROW_R, row.y + row.h / 2., uploads, 12., theme::text_muted());

    // App behavior
    let top = p.section(324., "App", ROW_H * 4.);
    let row = p.row(top, ROW_H, "Theme");
    let light = general.theme.eq_ignore_ascii_case("light");
    let segs = p.segmented(row, &["Dark", "Light"], usize::from(light));
    // Only the unselected segment flips the theme.
    p.hit(segs[usize::from(!light)], Action::ToggleTheme);
    p.divider(top + ROW_H);
    let row = p.row(top + ROW_H, ROW_H, "Keep capture history");
    p.switch(row, general.history_enabled, Action::ToggleHistory);
    p.divider(top + ROW_H * 2.);
    let row = p.row(top + ROW_H * 2., ROW_H, "Start with Windows");
    p.switch(row, autostart, Action::ToggleAutostart);
    p.divider(top + ROW_H * 3.);
    let row = p.row(top + ROW_H * 3., ROW_H, "When closed");
    let segs = p.segmented(
        row,
        &["Keep in tray", "Quit completely"],
        usize::from(general.quit_on_close),
    );
    p.hit(segs[0], Action::SetCloseBehavior(false));
    p.hit(segs[1], Action::SetCloseBehavior(true));
}

fn render_shortcuts_tab(p: &mut Painter, config: &Config, editing: Option<usize>) {
    let entries = config.shortcuts.entries();
    p.list_card(
        "Click a tool, then press a letter. Esc cancels.",
        entries.len(),
    );
    // Keys bound to more than one tool are rendered in red.
    let is_duplicate = |key: &str| -> bool {
        !key.is_empty()
            && entries
                .iter()
                .filter(|(_, _, k)| k.eq_ignore_ascii_case(key))
                .count()
                > 1
    };
    for (i, (_, label, key)) in entries.iter().enumerate() {
        let row = p.list_row(i, label);
        let active = editing == Some(i);
        let mid = row.y + row.h / 2.;
        let chip = Rect::new(ROW_R - 44., mid - 10., 44., 20.);
        let text = if active {
            "...".to_string()
        } else {
            key.to_uppercase()
        };
        let rgb = if !active && is_duplicate(key) {
            theme::red()
        } else {
            theme::text_normal()
        };
        let hovered = p.hover(row);
        p.key_chip(chip, &text, active, hovered, rgb);
        p.hit(row, Action::ClickShortcut(i));
    }
}

fn render_toolbar_tab(p: &mut Painter, config: &Config) {
    let entries = config.toolbar.entries();
    p.list_card("Choose which tools appear in the editor.", entries.len());
    for (i, (_, label, enabled)) in entries.iter().enumerate() {
        let row = p.list_row(i, label);
        let mid = row.y + row.h / 2.;
        let bx = Rect::new(ROW_R - 18., mid - 9., 18., 18.);
        if *enabled {
            p.c.round(bx, 5., theme::accent(), 1.);
            p.c.icon("check", bx.x + 3., bx.y + 3., 12., theme::text_on_accent());
        } else {
            p.c.round(bx, 5., theme::bg_4(), 0.5);
            p.c.outline(bx, 5., theme::border_strong(), 1.);
        }
        p.hit(row, Action::ToggleToolbar(i));
    }
}

impl SettingsWindow {
    pub fn new(
        window: Arc<Window>,
        surface: crate::native_surface::Surface,
        config: Config,
    ) -> Self {
        Self {
            window,
            surface,
            ui: SettingsUi::new(config),
            frame: None,
            // The caller and Surface::new have already styled the caption.
            styled: Some(theme::mode()),
        }
    }
    pub fn on_cursor_moved(&mut self, x: f32, y: f32) -> bool {
        let dpi = self.window.scale_factor() as f32;
        self.ui.on_logical_cursor_moved(x / dpi, y / dpi)
    }
    pub fn render(&mut self) {
        let size = self.window.inner_size();
        if size.width == 0 || size.height == 0 {
            return;
        }
        if self.frame.as_ref().map_or(true, |p| {
            (p.width(), p.height()) != (size.width, size.height)
        }) {
            self.frame = Pixmap::new(size.width, size.height);
        }
        let Some(frame) = &mut self.frame else {
            return;
        };
        self.ui.draw(frame);
        // DWM caption calls are only needed when the theme changes, not per hover.
        let mode = theme::mode();
        if self.styled != Some(mode) {
            crate::launcher::style_window(&self.window);
            self.styled = Some(mode);
        }
        let _ = self.surface.present(frame);
    }
}
