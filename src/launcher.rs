//! Compact native capture window and shared dropdown model.
use crate::{
    config::WorkflowConfig,
    geometry::Point,
    icons::IconCache,
    ui::{self, Canvas, Rect},
};
use std::{cell::RefCell, sync::Arc};
use tiny_skia::Pixmap;
use winit::{
    event_loop::ActiveEventLoop,
    window::{Window, WindowAttributes},
};
pub const WIDTH: u32 = 660;
pub const HEIGHT: u32 = 372;
pub const COMPACT_HEIGHT: u32 = 112;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CaptureMode {
    #[default]
    Region,
    Window,
    FullScreen,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    SizeDown,
    SizeUp,
    SetSize(u16),
    EditSize,
    DeleteAnnotation,
    New,
    Mode(CaptureMode),
    Delay,
    SetDelay(u32),
    Preview,
    AutoSave,
    ConfirmUpload,
    Settings,
    Quit,
    History,
    Upload,
    DismissUploadFeedback,
    UploadFeedbackBody,
    ConfirmPublicUpload,
    CancelUpload,
    Connect,
    SetHost(crate::upload::UploadHost),
    SaveConnection,
    CancelConnection,
    ToggleModes,
    ToggleDelay,
    ToggleOptions,
    Dismiss,
    Button(usize),
    Undo,
    Redo,
    ResetZoom,
    ToggleTools,
    ToggleColors,
    CustomColor,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Menu {
    Annotation,
    #[default]
    None,
    Modes,
    Delay,
    Options,
    UploadConfirm,
    Hosts,
    Tools,
    Colors,
}
#[derive(Clone, Copy)]
pub struct Target {
    pub rect: Rect,
    pub action: Action,
}
impl Target {
    pub fn new(x: f32, y: f32, w: f32, h: f32, action: Action) -> Self {
        Self {
            rect: Rect::new(x, y, w, h),
            action,
        }
    }
}
pub struct Model {
    pub mode: CaptureMode,
    pub delay: u32,
    pub workflow: WorkflowConfig,
    pub history_enabled: bool,
    pub has_image: bool,
    pub connected: bool,
    pub connecting: bool,
    pub draft: String,
    pub status: String,
    pub upload_feedback: crate::upload_feedback::UploadFeedback,
    pub hotkey: String,
    pub menu: Menu,
    pub hovered: Option<Action>,
    pub keyboard_focus: bool,
    pub icons: RefCell<IconCache>,
}
impl Model {
    pub fn new(workflow: WorkflowConfig, hotkey: String) -> Self {
        Self {
            mode: CaptureMode::Region,
            delay: 0,
            workflow,
            history_enabled: false,
            has_image: false,
            connected: false,
            connecting: false,
            draft: String::new(),
            status: String::new(),
            upload_feedback: Default::default(),
            hotkey,
            menu: Menu::None,
            hovered: None,
            keyboard_focus: false,
            icons: RefCell::new(IconCache::new()),
        }
    }
    /// The empty launcher is a command strip, not an unused image canvas.
    /// Expand for menus so every existing row remains reachable.
    pub fn height(&self) -> u32 {
        if self.connecting || self.menu != Menu::None {
            HEIGHT
        } else if self.upload_feedback.visible() {
            190
        } else {
            COMPACT_HEIGHT
        }
    }
    pub fn feedback_visible(&self) -> bool {
        self.upload_feedback.visible() && self.menu == Menu::None && !self.connecting
    }
    pub fn escape_dismisses_feedback(&self) -> bool {
        self.feedback_visible() && !self.upload_feedback.busy()
    }
    pub fn feedback_rect(&self, w: f32, editor: bool) -> Rect {
        Rect::new(w - if editor { 355. } else { 441. }, 68., 320., 110.)
    }
    pub fn feedback_targets(&self, w: f32, editor: bool) -> Vec<Target> {
        if !self.feedback_visible() {
            return Vec::new();
        }
        let r = self.feedback_rect(w, editor);
        let mut targets = Vec::new();
        if !self.upload_feedback.busy() {
            targets.push(Target::new(
                r.x + r.w - 28.,
                r.y + 7.,
                22.,
                22.,
                Action::DismissUploadFeedback,
            ));
        }
        targets.push(Target::new(r.x, r.y, r.w, r.h, Action::UploadFeedbackBody));
        targets
    }
    pub fn render_upload_feedback(&self, c: &mut Canvas, w: f32, editor: bool) {
        if !self.feedback_visible() {
            return;
        }
        let r = self.feedback_rect(w, editor);
        c.round(r, 10., ui::panel(), 1.);
        c.outline(r, 10., ui::focus_color(), 1.);
        c.label(
            r.x + 14.,
            r.y + 13.,
            self.upload_feedback.heading(),
            14.,
            ui::ink(),
        );
        if !self.upload_feedback.busy() {
            c.centered(r.x + r.w - 17., r.y + 9., "×", 17., ui::muted());
        }
        let scale = c.scale;
        let fit = |text: &str| {
            let mut value: String = text.chars().take(160).collect();
            let mut clipped = text.chars().count() > 160;
            while ui::measure(&value, 12. * scale) / scale > r.w - 42. {
                value.pop();
                clipped = true;
            }
            if clipped {
                value.push('…');
            }
            value
        };
        c.label(
            r.x + 14.,
            r.y + 43.,
            &fit(self.upload_feedback.detail()),
            12.,
            ui::ink(),
        );
        c.label(
            r.x + 14.,
            r.y + 76.,
            &fit(self.upload_feedback.hint()),
            12.,
            ui::muted(),
        );
    }
    pub fn bar_targets(&self, w: f32, editor: bool) -> Vec<Target> {
        let mut t = vec![
            Target::new(18., 15., 84., 32., Action::New),
            Target::new(114., 15., 128., 32., Action::ToggleModes),
            Target::new(250., 15., 95., 32., Action::ToggleDelay),
        ];
        if editor {
            t.extend([
                Target::new(w - 309., 15., 82., 32., Action::Button(22)),
                Target::new(w - 219., 15., 80., 32., Action::Button(23)),
                Target::new(w - 131., 15., 96., 32., Action::Upload),
                Target::new(w - 33., 15., 28., 32., Action::ToggleOptions),
            ]);
        } else {
            if self.has_image {
                t.push(Target::new(w - 153., 15., 96., 32., Action::Upload));
            }
            t.push(Target::new(w - 113., 15., 95., 32., Action::ToggleOptions));
            if let Some(upload) = t.iter_mut().find(|t| t.action == Action::Upload) {
                upload.rect.x = w - 217.;
            }
        }
        t
    }
    pub fn menu_targets(&self, w: f32) -> Vec<Target> {
        match self.menu {
            Menu::Hosts => crate::upload::UploadHost::ALL
                .iter()
                .enumerate()
                .map(|(i, host)| {
                    Target::new(
                        w - 350.,
                        82. + i as f32 * 40.,
                        332.,
                        40.,
                        Action::SetHost(*host),
                    )
                })
                .collect(),
            Menu::Modes => [
                CaptureMode::Region,
                CaptureMode::Window,
                CaptureMode::FullScreen,
            ]
            .iter()
            .enumerate()
            .map(|(i, m)| Target::new(114., 55. + i as f32 * 32., 178., 32., Action::Mode(*m)))
            .collect(),
            Menu::Delay => [0, 3, 5, 10]
                .iter()
                .enumerate()
                .map(|(i, s)| {
                    Target::new(250., 55. + i as f32 * 32., 152., 32., Action::SetDelay(*s))
                })
                .collect(),
            Menu::UploadConfirm => vec![
                Target::new((w - 360.) / 2. + 18., 194., 96., 32., Action::CancelUpload),
                Target::new(
                    (w - 360.) / 2. + 160.,
                    194.,
                    182.,
                    32.,
                    Action::ConfirmPublicUpload,
                ),
            ],
            Menu::Options => [
                (Action::History, 79.),
                (Action::Preview, 111.),
                (Action::AutoSave, 143.),
                (Action::ConfirmUpload, 199.),
                (Action::Connect, 231.),
                (Action::Settings, 287.),
                (Action::Quit, 323.),
            ]
            .iter()
            .map(|(a, y)| Target::new(w - 350., *y, 332., 32., *a))
            .collect(),
            _ => Vec::new(),
        }
    }
    fn targets(&self) -> Vec<Target> {
        if self.connecting {
            return vec![
                Target::new(104., 236., 146., 32., Action::SaveConnection),
                Target::new(260., 236., 86., 32., Action::CancelConnection),
            ];
        }
        if self.menu != Menu::None {
            self.menu_targets(WIDTH as f32)
        } else {
            let mut targets = self.feedback_targets(WIDTH as f32, false);
            targets.extend(self.bar_targets(WIDTH as f32, false));
            targets
        }
    }
    pub fn hit(&self, p: Point) -> Option<Action> {
        self.targets()
            .into_iter()
            .find(|t| t.rect.contains(p))
            .map(|t| t.action)
            .or_else(|| (self.menu != Menu::None).then_some(Action::Dismiss))
    }
    pub fn hover(&mut self, p: Point) -> bool {
        let was_focused = self.keyboard_focus;
        self.keyboard_focus = false;
        let h = self.hit(p).filter(|a| *a != Action::Dismiss);
        let changed = was_focused || h != self.hovered;
        self.hovered = h;
        changed
    }
    pub fn navigate(&mut self, direction: i32, w: f32) {
        let targets = if self.menu == Menu::None {
            self.bar_targets(w, false)
        } else {
            self.menu_targets(w)
        };
        step_focus(self, &targets, direction);
    }
    pub fn enter(&self, w: f32) -> Option<Action> {
        if self.menu != Menu::None {
            self.hovered
                .filter(|a| self.menu_targets(w).iter().any(|t| t.action == *a))
                .or_else(|| self.menu_targets(w).first().map(|t| t.action))
        } else if self.keyboard_focus {
            self.hovered
        } else {
            Some(Action::New)
        }
    }
    pub fn activate(&mut self, a: Action) -> bool {
        let next = match a {
            Action::Preview | Action::AutoSave | Action::ConfirmUpload => return false,
            Action::ToggleModes => Menu::Modes,
            Action::ToggleDelay => Menu::Delay,
            Action::ToggleOptions => Menu::Options,
            Action::Connect => Menu::Hosts,
            Action::SetHost(host) => {
                self.workflow.upload_host = host;
                self.menu = Menu::Options;
                return false;
            }
            Action::Dismiss => Menu::None,
            _ => {
                self.menu = Menu::None;
                return false;
            }
        };
        self.menu = if self.menu == next { Menu::None } else { next };
        true
    }
    pub fn cycle_delay(&mut self) {
        self.delay = match self.delay {
            0 => 3,
            3 => 5,
            5 => 10,
            _ => 0,
        };
    }
    pub fn append(&mut self, text: &str) {
        for c in text.chars().filter(|c| c.is_ascii_alphanumeric()) {
            if self.draft.len() < 128 {
                self.draft.push(c);
            }
        }
    }
    pub fn render_bar(&self, c: &mut Canvas, w: f32, editor: bool) {
        // Match the native caption exactly; frost remains in the workspace below.
        c.round(Rect::new(0., 0., w, 62.), 0., ui::base(), 1.0);
        c.round(Rect::new(0., 61., w, 1.), 0., ui::muted(), 0.10);
        let h = |a| self.hovered == Some(a);
        c.button(
            Rect::new(18., 15., 84., 32.),
            "New",
            Some("plus"),
            true,
            h(Action::New),
            true,
        );
        let name = match self.mode {
            CaptureMode::Region => "Rectangle",
            CaptureMode::Window => "Window",
            CaptureMode::FullScreen => "Full screen",
        };
        c.button(
            Rect::new(114., 15., 128., 32.),
            name,
            Some(match self.mode {
                CaptureMode::Region => "crop",
                CaptureMode::Window => "window",
                CaptureMode::FullScreen => "screen",
            }),
            false,
            h(Action::ToggleModes),
            true,
        );
        c.icon("down", 224., 24., 14., ui::muted());
        c.button(
            Rect::new(250., 15., 95., 32.),
            &if self.delay == 0 {
                "Delay".into()
            } else {
                format!("{} s", self.delay)
            },
            Some("clock"),
            false,
            h(Action::ToggleDelay),
            true,
        );
        c.icon("down", 328., 24., 14., ui::muted());
        if editor {
            c.button(
                Rect::new(w - 309., 15., 82., 32.),
                "Copy",
                Some("copy"),
                false,
                h(Action::Button(22)),
                true,
            );
            c.button(
                Rect::new(w - 219., 15., 80., 32.),
                "Save",
                Some("save"),
                false,
                h(Action::Button(23)),
                true,
            );
            c.button(
                Rect::new(w - 131., 15., 96., 32.),
                self.upload_feedback.button_label(),
                if self.upload_feedback.visible() {
                    None
                } else {
                    Some("upload")
                },
                self.upload_feedback.visible(),
                h(Action::Upload),
                true,
            );
            if h(Action::ToggleOptions) {
                c.round(Rect::new(w - 33., 15., 30., 32.), 6., (255, 255, 255), 0.09);
            }
            c.icon("gear", w - 26., 22., 18., ui::muted());
        } else {
            c.button(
                Rect::new(w - 217., 15., 96., 32.),
                self.upload_feedback.button_label(),
                if self.upload_feedback.visible() {
                    None
                } else {
                    Some("upload")
                },
                self.upload_feedback.visible(),
                h(Action::Upload),
                self.has_image,
            );
            c.button(
                Rect::new(w - 113., 15., 95., 32.),
                "Options",
                Some("gear"),
                false,
                h(Action::ToggleOptions),
                true,
            );
        }
    }
    pub fn render_focus(&self, c: &mut Canvas, targets: &[Target]) {
        if self.keyboard_focus {
            if let Some(t) = targets.iter().find(|t| Some(t.action) == self.hovered) {
                c.outline(
                    Rect::new(t.rect.x - 2., t.rect.y - 2., t.rect.w + 4., t.rect.h + 4.),
                    7.,
                    ui::focus_color(),
                    1.5,
                );
            }
        }
    }
    pub fn render_menu(&self, c: &mut Canvas, w: f32) {
        if self.menu == Menu::UploadConfirm {
            let x = (w - 360.) / 2.;
            c.round(Rect::new(x, 82., 360., 158.), 12., ui::panel(), 1.);
            c.icon("upload", x + 18., 99., 20., ui::accent());
            c.label(x + 48., 99., "Create a public image link?", 16., ui::ink());
            c.label(
                x + 18.,
                137.,
                &format!(
                    "Your image will be uploaded to {}.",
                    self.workflow.upload_host.label()
                ),
                13.,
                ui::ink(),
            );
            c.label(
                x + 18.,
                159.,
                self.workflow.upload_host.disclosure(),
                12.,
                ui::muted(),
            );
            for t in self.menu_targets(w) {
                c.button(
                    t.rect,
                    if t.action == Action::CancelUpload {
                        "Cancel"
                    } else {
                        "Upload publicly"
                    },
                    None,
                    t.action == Action::ConfirmPublicUpload,
                    self.hovered == Some(t.action),
                    true,
                );
            }
            self.render_focus(c, &self.menu_targets(w));
            return;
        }
        let targets = self.menu_targets(w);
        if targets.is_empty() {
            return;
        }
        let first = targets[0].rect;
        let top = if self.menu == Menu::Options || self.menu == Menu::Hosts {
            55.
        } else {
            first.y - 4.
        };
        c.round(
            Rect::new(
                first.x - 4.,
                top,
                first.w + 8.,
                targets.last().unwrap().rect.y + targets.last().unwrap().rect.h + 4. - top,
            ),
            10.,
            ui::panel(),
            1.0,
        );
        if self.menu == Menu::Options {
            for (label, y) in [("Capture", 60.), ("Sharing", 180.), ("App", 268.)] {
                c.label(first.x + 12., y, label, 11., ui::muted());
            }
            c.round(
                Rect::new(first.x + 10., 321., first.w - 20., 1.),
                0.,
                ui::muted(),
                0.16,
            );
        }
        if self.menu == Menu::Hosts {
            c.label(
                first.x + 12.,
                61.,
                "Image host · No setup needed",
                11.,
                ui::muted(),
            );
        }
        self.render_focus(c, &targets);
        for t in targets {
            if self.hovered == Some(t.action) {
                c.round(t.rect, 5., (255, 255, 255), 0.09);
            }
            let (label, selected) = match t.action {
                Action::Mode(m) => (
                    match m {
                        CaptureMode::Region => "Rectangle".into(),
                        CaptureMode::Window => "Window".into(),
                        CaptureMode::FullScreen => "Full screen".into(),
                    },
                    m == self.mode,
                ),
                Action::SetDelay(s) => (
                    if s == 0 {
                        "No delay".into()
                    } else {
                        format!("{} seconds", s)
                    },
                    s == self.delay,
                ),
                Action::Preview => (
                    "Open editor after capture".into(),
                    self.workflow.open_preview,
                ),
                Action::AutoSave => ("Auto-save screenshots".into(), self.workflow.auto_save),
                Action::ConfirmUpload => (
                    "Confirm before uploading".into(),
                    self.workflow.confirm_upload,
                ),
                Action::Connect => ("Image host".into(), false),
                Action::SetHost(host) => (host.label().into(), self.workflow.upload_host == host),
                Action::History => ("Capture history".into(), false),
                Action::Settings => ("Settings & appearance".into(), false),
                Action::Quit => ("Quit Snipping Tool UL".into(), false),
                _ => continue,
            };
            let icon = match t.action {
                Action::Mode(CaptureMode::Region) => "crop",
                Action::Mode(CaptureMode::Window) => "window",
                Action::Mode(CaptureMode::FullScreen) => "screen",
                Action::SetDelay(_) => "clock",
                Action::Preview => "preview",
                Action::AutoSave => "save",
                Action::ConfirmUpload | Action::Connect | Action::SetHost(_) => "upload",
                Action::History => "history",
                Action::Quit => "power",
                _ => "gear",
            };
            c.icon(icon, t.rect.x + 10., t.rect.y + 8., 16., ui::muted());
            c.label(
                t.rect.x + 36.,
                t.rect.y + if self.menu == Menu::Hosts { 3. } else { 7. },
                &label,
                13.,
                ui::ink(),
            );
            if let Action::SetHost(host) = t.action {
                let detail = match host {
                    crate::upload::UploadHost::Catbox => "No account · Original PNG",
                    crate::upload::UploadHost::Freeimage => {
                        "Public sharing · May appear on the host's site"
                    }
                    _ => "Temporary · Deletes automatically",
                };
                c.label(t.rect.x + 36., t.rect.y + 22., detail, 10., ui::muted());
            }
            if matches!(
                t.action,
                Action::Preview | Action::AutoSave | Action::ConfirmUpload
            ) {
                let r = Rect::new(t.rect.x + t.rect.w - 40., t.rect.y + 8., 30., 16.);
                c.round(
                    r,
                    8.,
                    if selected {
                        ui::accent()
                    } else {
                        crate::theme::bg_3()
                    },
                    1.,
                );
                c.round(
                    Rect::new(r.x + if selected { 16. } else { 2. }, r.y + 2., 12., 12.),
                    6.,
                    if selected {
                        crate::theme::text_on_accent()
                    } else {
                        ui::muted()
                    },
                    1.,
                );
                c.label(
                    r.x - 26.,
                    r.y + 1.,
                    if selected { "On" } else { "Off" },
                    11.,
                    ui::muted(),
                );
            } else if t.action == Action::Connect {
                c.label(
                    t.rect.x + t.rect.w
                        - 28.
                        - ui::measure(self.workflow.upload_host.name(), 12. * c.scale) / c.scale,
                    t.rect.y + 8.,
                    self.workflow.upload_host.name(),
                    12.,
                    ui::muted(),
                );
                c.icon(
                    "right",
                    t.rect.x + t.rect.w - 23.,
                    t.rect.y + 9.,
                    14.,
                    ui::muted(),
                );
            } else if t.action == Action::Settings || t.action == Action::History {
                if t.action == Action::History {
                    let r = Rect::new(t.rect.x + t.rect.w - 122., t.rect.y + 6., 94., 20.);
                    c.round(r, 6., crate::theme::bg_3(), 1.);
                    c.centered_in(
                        r,
                        if self.history_enabled {
                            "Currently on"
                        } else {
                            "Currently off"
                        },
                        11.,
                        if self.history_enabled {
                            ui::focus_color()
                        } else {
                            ui::muted()
                        },
                    );
                }
                c.icon(
                    "right",
                    t.rect.x + t.rect.w - 23.,
                    t.rect.y + 9.,
                    14.,
                    ui::muted(),
                );
            } else if selected {
                c.icon(
                    "check",
                    t.rect.x + t.rect.w - 26.,
                    t.rect.y + 9.,
                    14.,
                    (149, 198, 169),
                );
            }
        }
    }
    pub fn render_hint(&self, c: &mut Canvas, w: f32, editor: bool) {
        if self.menu != Menu::None
            || self.connecting
            || (self.hovered == Some(Action::Upload) && self.upload_feedback.visible())
        {
            return;
        }
        if let Some(t) = self
            .bar_targets(w, editor)
            .iter()
            .find(|t| Some(t.action) == self.hovered)
        {
            let text = match t.action {
                Action::New => "Start a new capture",
                Action::ToggleModes => "Choose rectangle, window, or full screen",
                Action::ToggleDelay => "Wait before starting a capture",
                Action::Upload => "Upload publicly and copy a share link",
                Action::ToggleOptions => "Capture preferences, sharing, and settings",
                Action::Button(22) => "Copy image with annotations (Ctrl+C)",
                Action::Button(23) => "Save image with annotations (Ctrl+S)",
                _ => return,
            };
            c.hint(t.rect, text, false);
        }
    }
    pub fn render(&self, p: &mut Pixmap) {
        ui::glass_fill(p);
        let scale = p.width() as f32 / WIDTH as f32;
        let mut icons = self.icons.borrow_mut();
        let mut c = Canvas {
            pixmap: p,
            scale,
            icons: &mut icons,
        };
        self.render_bar(&mut c, WIDTH as f32, false);
        if self.connecting {
            c.label(104., 103., "Optional Imgur connection", 19., ui::ink());
            c.label(
                104.,
                137.,
                "Default uploads need no setup. Clear this field to use Catbox.",
                12.,
                ui::muted(),
            );
            c.round(Rect::new(104., 171., 452., 40.), 7., ui::panel(), 0.85);
            c.label(
                116.,
                183.,
                &format!("{}|", self.draft.chars().take(48).collect::<String>()),
                14.,
                ui::ink(),
            );
            c.button(
                Rect::new(104., 236., 146., 32.),
                "Save connection",
                None,
                true,
                self.hovered == Some(Action::SaveConnection),
                true,
            );
            c.button(
                Rect::new(260., 236., 86., 32.),
                "Cancel",
                None,
                false,
                self.hovered == Some(Action::CancelConnection),
                true,
            );
        } else if self.menu == Menu::None {
            let shortcut = format!("Shortcut  {}", self.hotkey);
            c.label(20., 77., &shortcut, 12., ui::ink());
            let label = if self.status.is_empty() {
                if self.workflow.open_preview {
                    "Copies immediately · Opens preview"
                } else {
                    "Copies immediately · Preview off"
                }
            } else {
                &self.status
            };
            // Bound the status so it cannot collide with long/custom hotkeys.
            let available =
                (638. - 20. - ui::measure(&shortcut, 12. * scale) / scale - 24.).max(0.);
            let mut label: String = label.chars().take(48).collect();
            while ui::measure(&label, 11. * scale) / scale > available {
                label.pop();
            }
            let width = ui::measure(&label, 11. * scale) / scale;
            if !self.upload_feedback.visible() {
                c.label(638. - width, 78., &label, 11., ui::muted());
            }
        }
        if !self.connecting {
            self.render_menu(&mut c, WIDTH as f32);
            self.render_hint(&mut c, WIDTH as f32, false);
            self.render_upload_feedback(&mut c, WIDTH as f32, false);
            if self.menu == Menu::None {
                self.render_focus(&mut c, &self.bar_targets(WIDTH as f32, false));
            }
        }
    }
}
pub fn step_focus(model: &mut Model, targets: &[Target], direction: i32) {
    let targets: Vec<_> = targets
        .iter()
        .filter(|t| t.action != Action::UploadFeedbackBody)
        .copied()
        .collect();
    if targets.is_empty() {
        return;
    }
    let n = targets.len() as i32;
    let i = targets
        .iter()
        .position(|t| Some(t.action) == model.hovered)
        .map(|i| i as i32);
    let next = match i {
        Some(i) => (i + direction).rem_euclid(n),
        None => {
            if direction < 0 {
                n - 1
            } else {
                0
            }
        }
    };
    model.hovered = Some(targets[next as usize].action);
    model.keyboard_focus = true;
}
pub struct LauncherWindow {
    pub window: Arc<Window>,
    surface: crate::native_surface::Surface,
    pixmap: Pixmap,
    pub model: Model,
    pub pointer: Point,
}
impl LauncherWindow {
    pub fn create(event_loop: &ActiveEventLoop, model: Model) -> Result<Self, String> {
        let window = Arc::new(
            event_loop
                .create_window(
                    WindowAttributes::default()
                        .with_title("Snipping Tool UL")
                        .with_inner_size(winit::dpi::LogicalSize::new(WIDTH, model.height()))
                        .with_resizable(false)
                        .with_visible(false),
                )
                .map_err(|e| e.to_string())?,
        );
        let surface = crate::native_surface::Surface::new(window.clone())?;
        let height = model.height();
        Ok(Self {
            window,
            surface,
            pixmap: Pixmap::new(WIDTH, height).ok_or("Home allocation failed")?,
            model,
            pointer: Point::new(-1., -1.),
        })
    }
    pub fn render(&mut self) -> Result<(), String> {
        let desired = winit::dpi::LogicalSize::new(WIDTH, self.model.height())
            .to_physical::<u32>(self.window.scale_factor());
        if self.window.inner_size() != desired {
            let _ = self.window.request_inner_size(desired);
        }
        let s = self.window.inner_size();
        if s.width == 0 || s.height == 0 {
            return Ok(());
        }
        if (self.pixmap.width(), self.pixmap.height()) != (s.width, s.height) {
            self.pixmap = Pixmap::new(s.width, s.height).ok_or("Home allocation failed")?;
        }
        self.model.render(&mut self.pixmap);
        self.surface.present(&self.pixmap)
    }
    pub fn point(&self, x: f64, y: f64) -> Point {
        let s = self.window.inner_size();
        // Match Canvas's width-based DPI scale, including a pending height change.
        let scale = s.width.max(1) as f64 / WIDTH as f64;
        Point::new((x / scale) as f32, (y / scale) as f32)
    }
}
pub fn style_window(window: &Window) -> bool {
    window.set_window_icon(window_icon());
    let mut caption_theme_applied = false;
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::{Foundation::HWND, Graphics::Dwm::*};
        use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
        if let Ok(handle) = window.window_handle() {
            if let RawWindowHandle::Win32(h) = handle.as_raw() {
                unsafe {
                    let hwnd = HWND(h.hwnd.get() as _);
                    let dark = u32::from(crate::theme::mode() == crate::theme::ThemeMode::Dark);
                    let corners = 2u32;
                    let _ = DwmSetWindowAttribute(
                        hwnd,
                        DWMWA_USE_IMMERSIVE_DARK_MODE,
                        &dark as *const _ as _,
                        4,
                    );
                    // Respect system high contrast rather than overriding its caption colors.
                    use windows::Win32::UI::{Accessibility::*, WindowsAndMessaging::*};
                    let mut high = HIGHCONTRASTW {
                        cbSize: std::mem::size_of::<HIGHCONTRASTW>() as u32,
                        ..Default::default()
                    };
                    let high_contrast = SystemParametersInfoW(
                        SPI_GETHIGHCONTRAST,
                        high.cbSize,
                        Some(&mut high as *mut _ as _),
                        SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
                    )
                    .is_ok()
                        && high.dwFlags & HCF_HIGHCONTRASTON != HIGHCONTRASTW_FLAGS(0);
                    if !high_contrast {
                        caption_theme_applied = true;
                        let color_ref = |rgb: (u8, u8, u8)| {
                            rgb.0 as u32 | (rgb.1 as u32) << 8 | (rgb.2 as u32) << 16
                        };
                        // Windows' default acrylic caption can become warm gray.
                        // Keep a consistent themed header with either presenter.
                        let caption = color_ref(ui::base());
                        for (attribute, color) in [
                            (DWMWA_CAPTION_COLOR, caption),
                            (DWMWA_TEXT_COLOR, color_ref(ui::ink())),
                            (DWMWA_BORDER_COLOR, color_ref(ui::base())),
                        ] {
                            caption_theme_applied &=
                                DwmSetWindowAttribute(hwnd, attribute, &color as *const _ as _, 4)
                                    .is_ok();
                        }
                    }
                    let _ = DwmSetWindowAttribute(
                        hwnd,
                        DWMWA_WINDOW_CORNER_PREFERENCE,
                        &corners as *const _ as _,
                        4,
                    );
                }
            }
        }
    }
    #[cfg(not(target_os = "windows"))]
    let _ = window;
    caption_theme_applied
}
/// Pre-rendered authored logo: no SVG parsing or PNG decode during window creation.
pub fn window_icon() -> Option<winit::window::Icon> {
    static ICON: std::sync::LazyLock<Option<winit::window::Icon>> =
        std::sync::LazyLock::new(|| {
            winit::window::Icon::from_rgba(logo_rgba().to_vec(), 64, 64).ok()
        });
    ICON.clone()
}
pub fn logo_rgba() -> &'static [u8; 64 * 64 * 4] {
    include_bytes!("../assets/ul-icon.rgba")
}
