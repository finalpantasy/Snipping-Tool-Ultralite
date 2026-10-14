//! Editor chrome is drawn in viewport coordinates, independently of capture pixels.
use crate::{
    geometry::Point,
    launcher::{Action, Menu, Model, Target},
    overlay::toolbar::{ButtonAction, BUTTONS},
    state::OverlayState,
    ui::{self, Canvas, Rect},
};
pub const MIN_WIDTH: u32 = 660;
pub const MIN_HEIGHT: u32 = 390;
pub const PRIMARY: [usize; 7] = [6, 7, 9, 1, 2, 10, 0];
fn tool_label(i: usize) -> &'static str {
    match i {
        0 => "Select",
        2 => "Box",
        4 => "Rounded box",
        7 => "Highlight",
        11 => "Number steps",
        12 => "Pick a color",
        13 => "Measure",
        19 => "Extract text",
        21 => "Pin on screen",
        _ => BUTTONS[i]
            .tooltip
            .split(" (")
            .next()
            .unwrap_or(BUTTONS[i].tooltip),
    }
}
fn tool_hints(config: &crate::config::Config) -> Vec<String> {
    let descriptions = [
        "select or move an annotation",
        "draw an arrow",
        "draw a rectangle",
        "draw a circle",
        "draw a rounded rectangle",
        "draw a line",
        "draw freehand",
        "highlight a detail",
        "dim everything around a detail",
        "add text",
        "obscure image details",
        "add numbered steps",
        "sample a color from the image",
        "measure a distance in pixels",
    ];
    config
        .shortcuts
        .entries()
        .iter()
        .enumerate()
        .map(|(i, (_, label, key))| {
            format!("{} ({}): {}", label, key.to_uppercase(), descriptions[i])
        })
        .collect()
}
pub struct EditorUi {
    pub header: Model,
    pub pointer: Point,
    tool_hints: Vec<String>,
    pub size_draft: Option<String>,
    pub size_select_all: bool,
    pub size_dragging: bool,
    size_origin: Option<(usize, crate::tools::Annotation)>,
    pub annotation_anchor: Point,
    new_capture_armed: bool,
    /// The glass fill plus the canvas well, which never change between
    /// frames at a given size, scale and theme. Re-blending them cost
    /// 2.4 ms per frame at the default window and 20 ms at 4K.
    base_cache: std::cell::RefCell<Option<BaseCache>>,
}

/// The chrome is mostly repeated rows (flat glass, the canvas well's sides),
/// so it is kept as one copy of each run of identical rows: about 0.14 MB
/// instead of a 5 MB full frame at the default size, 0.4 MB instead of 33 MB
/// at 4K. Replaying it is a row copy per row and byte-identical.
struct BaseCache {
    key: (u32, u32, u32, crate::theme::ThemeMode, u8),
    /// One stored row per run, and how many consecutive rows repeat it.
    rows: Vec<u8>,
    runs: Vec<u32>,
}
impl BaseCache {
    fn new(key: (u32, u32, u32, crate::theme::ThemeMode, u8), data: &[u8], stride: usize) -> Self {
        let (mut rows, mut runs) = (Vec::new(), Vec::<u32>::new());
        let mut last: Option<&[u8]> = None;
        for row in data.chunks_exact(stride) {
            match (last, runs.last_mut()) {
                (Some(prev), Some(n)) if prev == row => *n += 1,
                _ => {
                    rows.extend_from_slice(row);
                    runs.push(1);
                    last = Some(row);
                }
            }
        }
        rows.shrink_to_fit();
        Self { key, rows, runs }
    }
    fn restore(&self, data: &mut [u8], stride: usize) {
        let mut out = data.chunks_exact_mut(stride);
        for (row, n) in self.rows.chunks_exact(stride).zip(&self.runs) {
            for dst in out.by_ref().take(*n as usize) {
                dst.copy_from_slice(row);
            }
        }
    }
    /// Bytes held, for memory measurements.
    fn bytes(&self) -> usize {
        self.rows.capacity() + self.runs.capacity() * 4
    }
}
impl EditorUi {
    pub fn new(config: &crate::config::Config) -> Self {
        let mut header = Model::new(config.workflow.clone(), config.hotkey.capture.clone());
        header.has_image = true;
        header.history_enabled = config.general.history_enabled;
        header.connected = !config.general.imgur_client_id.is_empty()
            || std::env::var("SNIPPING_TOOL_UL_IMGUR_CLIENT_ID").is_ok_and(|v| !v.is_empty());
        Self {
            header,
            pointer: Point::new(-1., -1.),
            tool_hints: tool_hints(config),
            size_draft: None,
            size_select_all: false,
            size_dragging: false,
            size_origin: None,
            annotation_anchor: Point::new(0., 0.),
            new_capture_armed: false,
            base_cache: std::cell::RefCell::new(None),
        }
    }
    pub fn image_rect(w: f32, h: f32) -> Rect {
        Rect::new(46., 96., (w - 92.).max(1.), (h - 264.).max(1.))
    }
    pub fn arm_new_capture(&mut self) {
        self.new_capture_armed = true;
    }
    pub fn new_capture_armed(&self) -> bool {
        self.new_capture_armed
    }
    pub fn cancel_new_capture(&mut self) {
        self.new_capture_armed = false;
    }
    /// Release the initiating click before transferring input to a new picker.
    pub fn release_new_capture(
        &mut self,
        p: Point,
        w: f32,
        h: f32,
        state: &OverlayState,
    ) -> Option<Action> {
        (std::mem::take(&mut self.new_capture_armed)
            && self.hit(p, w, h, state) == Some(Action::New))
        .then_some(Action::New)
    }
    pub fn sync_preferences(&mut self, config: &crate::config::Config) {
        self.header.workflow = config.workflow.clone();
        self.header.history_enabled = config.general.history_enabled;
        self.header.connected = !config.general.imgur_client_id.is_empty()
            || std::env::var("SNIPPING_TOOL_UL_IMGUR_CLIENT_ID").is_ok_and(|v| !v.is_empty());
        self.tool_hints = tool_hints(config);
    }
    pub fn canvas_rect(w: f32, h: f32) -> Rect {
        Rect::new(18., 68., w - 36., (h - 208.).max(1.))
    }
    fn context_x(w: f32) -> f32 {
        (w - 349.) / 2.
    }
    pub fn size_slider(w: f32, h: f32, _state: &OverlayState) -> Rect {
        let x = Self::context_x(w);
        Rect::new(x + 180., h - 130., 130., 30.)
    }
    pub fn slider_value(p: Point, w: f32, h: f32, state: &OverlayState) -> Option<Action> {
        if state.size_kind() == crate::editing::SizeKind::None {
            return None;
        }
        let r = Self::size_slider(w, h, state);
        let (lo, hi) = state.size_kind().limits();
        let value = (lo + (hi - lo) * ((p.x - r.x) / r.w).clamp(0., 1.)).round();
        Some(Action::SetSize((value * 10.) as u16))
    }
    pub fn begin_size_drag(&mut self, state: &OverlayState) {
        self.size_dragging = true;
        self.size_origin = state
            .selected_index
            .and_then(|i| state.annotations.get(i).map(|a| (i, a.clone())));
    }
    pub fn end_size_drag(&mut self, state: &mut OverlayState) {
        self.size_dragging = false;
        if let Some((i, old)) = self.size_origin.take() {
            if state.annotations.get(i).is_some_and(|a| *a != old) {
                crate::tools::record_undo(
                    &mut state.undo_stack,
                    &mut state.redo_stack,
                    crate::tools::UndoAction::Modify(i, old),
                );
            }
        }
    }
    fn context_targets(&self, w: f32, h: f32, state: &OverlayState) -> Vec<Target> {
        let mut targets = Vec::new();
        let x = Self::context_x(w);
        let y = h - 130.;
        if state.size_kind() != crate::editing::SizeKind::None {
            targets.extend([
                Target::new(x + 66., y, 28., 30., Action::SizeDown),
                Target::new(x + 100., y, 68., 30., Action::EditSize),
                Target::new(x + 321., y, 28., 30., Action::SizeUp),
                Target::new(
                    x + 180.,
                    y,
                    130.,
                    30.,
                    Action::SetSize((state.tool_size() * 10.) as u16),
                ),
            ]);
        }
        targets
    }
    fn targets(&self, w: f32, h: f32, state: &OverlayState) -> Vec<Target> {
        let tx = (w - 612.) / 2.;
        let ty = h - 90.;
        if self.header.menu == Menu::Annotation {
            return vec![Target::new(
                // A minimized window is 0x0; keep the bounds ordered.
                self.annotation_anchor.x.clamp(18., (w - 208.).max(18.)),
                self.annotation_anchor.y.clamp(55., (h - 180.).max(55.)),
                190.,
                34.,
                Action::DeleteAnnotation,
            )];
        }
        if self.header.menu == Menu::Tools {
            let items: Vec<_> = state
                .visible_buttons
                .iter()
                .copied()
                .filter(|i| {
                    !PRIMARY.contains(i)
                        && *i != 20
                        && *i != 22
                        && *i != 23
                        && (*i < 14 || *i == 19 || *i == 21)
                })
                .collect();
            let annotations: Vec<_> = items.iter().copied().filter(|i| *i < 14).collect();
            let actions: Vec<_> = items.iter().copied().filter(|i| *i >= 19).collect();
            let rows = annotations.len().div_ceil(2) + actions.len().div_ceil(2);
            let mut y = (ty - rows as f32 * 32. - 58.).max(55.);
            let mut targets = Vec::new();
            for group in [annotations, actions] {
                if group.is_empty() {
                    continue;
                }
                y += 24.;
                for (n, i) in group.iter().enumerate() {
                    targets.push(Target::new(
                        (w - 360.) / 2. + (n % 2) as f32 * 180.,
                        y + (n / 2) as f32 * 32.,
                        180.,
                        32.,
                        Action::Button(*i),
                    ));
                }
                y += group.len().div_ceil(2) as f32 * 32.;
            }
            return targets;
        }
        if self.header.menu == Menu::Colors {
            let mut colors: Vec<_> = (0..5)
                .map(|i| {
                    Target::new(
                        tx + 394.,
                        ty - 204. + i as f32 * 32.,
                        180.,
                        32.,
                        Action::Button(14 + i),
                    )
                })
                .collect();
            colors.push(Target::new(
                tx + 394.,
                ty - 44.,
                180.,
                32.,
                Action::CustomColor,
            ));
            return colors;
        }
        if self.header.menu != Menu::None {
            return self.header.menu_targets(w);
        }
        let mut t = self.header.feedback_targets(w, true);
        t.extend(self.header.bar_targets(w, true));
        t.extend(self.context_targets(w, h, state));
        t.extend([
            Target::new(tx + 8., ty + 7., 34., 46., Action::Undo),
            Target::new(tx + 45., ty + 7., 34., 46., Action::Redo),
        ]);
        for (n, i) in PRIMARY.iter().enumerate() {
            if state.visible_buttons.contains(i) {
                t.push(Target::new(
                    tx + 100. + n as f32 * 45.,
                    ty + 7.,
                    40.,
                    46.,
                    Action::Button(*i),
                ));
            }
        }
        t.extend([
            Target::new(tx + 442., ty + 7., 48., 46., Action::ToggleColors),
            Target::new(tx + 498., ty + 7., 48., 46., Action::ToggleTools),
            Target::new(tx + 552., ty + 7., 52., 46., Action::ResetZoom),
        ]);
        t
    }
    pub fn hit(&self, p: Point, w: f32, h: f32, state: &OverlayState) -> Option<Action> {
        if self.header.menu == Menu::None && Self::size_slider(w, h, state).contains(p) {
            if let Some(a) = Self::slider_value(p, w, h, state) {
                return Some(a);
            }
        }
        self.targets(w, h, state)
            .into_iter()
            .find(|t| t.rect.contains(p))
            .map(|t| t.action)
            .or_else(|| (self.header.menu != Menu::None).then_some(Action::Dismiss))
    }
    pub fn hover(&mut self, p: Point, w: f32, h: f32, state: &OverlayState) -> bool {
        let was_focused = self.header.keyboard_focus;
        self.header.keyboard_focus = false;
        self.pointer = p;
        let next = self.hit(p, w, h, state).filter(|a| *a != Action::Dismiss);
        let changed = was_focused || next != self.header.hovered;
        self.header.hovered = next;
        changed
    }
    pub fn navigate(&mut self, w: f32, h: f32, state: &OverlayState, direction: i32) {
        let targets = self.targets(w, h, state);
        crate::launcher::step_focus(&mut self.header, &targets, direction);
    }
    pub fn enter(&self, w: f32, h: f32, state: &OverlayState) -> Option<Action> {
        let targets = self.targets(w, h, state);
        self.header
            .hovered
            .filter(|a| targets.iter().any(|t| t.action == *a))
            .or_else(|| {
                (self.header.menu != Menu::None)
                    .then(|| targets.first().map(|t| t.action))
                    .flatten()
            })
    }
    pub fn activate(&mut self, a: Action) -> bool {
        match a {
            Action::ToggleTools => {
                self.header.menu = if self.header.menu == Menu::Tools {
                    Menu::None
                } else {
                    Menu::Tools
                };
                true
            }
            Action::ToggleColors => {
                self.header.menu = if self.header.menu == Menu::Colors {
                    Menu::None
                } else {
                    Menu::Colors
                };
                true
            }
            _ => self.header.activate(a),
        }
    }
    pub fn draw_base(&self, p: &mut tiny_skia::Pixmap, dpi: f32) {
        let key = (
            p.width(),
            p.height(),
            dpi.to_bits(),
            crate::theme::mode(),
            crate::theme::glass_alpha(),
        );
        // The key includes width and height, so a hit always fits the frame.
        let stride = p.width() as usize * 4;
        let mut cached = self.base_cache.borrow_mut();
        if let Some(cache) = cached.as_ref().filter(|c| c.key == key) {
            cache.restore(p.data_mut(), stride);
            return;
        }
        ui::glass_fill(p);
        let (w, h) = (p.width() as f32 / dpi, p.height() as f32 / dpi);
        let mut icons = self.header.icons.borrow_mut();
        let mut c = Canvas {
            pixmap: p,
            scale: dpi,
            icons: &mut icons,
        };
        c.round(Self::canvas_rect(w, h), 8., crate::theme::bg_0(), 0.20);
        *cached = Some(BaseCache::new(key, p.data(), stride));
    }

    /// Bytes held by the chrome cache, for memory measurements.
    pub fn frame_cache_bytes(&self) -> usize {
        self.base_cache
            .borrow()
            .as_ref()
            .map_or(0, BaseCache::bytes)
    }

    /// Free the cached chrome layer; it must not outlive the editor window.
    pub fn release_frame_cache(&mut self) {
        *self.base_cache.get_mut() = None;
    }
    pub fn draw_controls(
        &self,
        p: &mut tiny_skia::Pixmap,
        dpi: f32,
        state: &mut OverlayState,
        zoom: f64,
    ) {
        let (w, h) = (p.width() as f32 / dpi, p.height() as f32 / dpi);
        let mut cache = self.header.icons.borrow_mut();
        let mut c = Canvas {
            pixmap: p,
            scale: dpi,
            icons: &mut cache,
        };
        self.header.render_bar(&mut c, w, true);
        let cx = Self::context_x(w);
        let cy = h - 130.;
        let label = if state.size_kind() == crate::editing::SizeKind::None {
            match state.active_tool {
                crate::tools::ToolKind::Highlight => "Drag a box over words",
                crate::tools::ToolKind::Eyedropper => "Click image to pick a color",
                crate::tools::ToolKind::Measurement => "Drag to measure",
                crate::tools::ToolKind::Select => {
                    "Select an annotation · Drag to move · Corners resize"
                }
                _ => "Drag a box to spotlight",
            }
        } else {
            state.size_kind().label()
        };
        c.centered_in(
            Rect::new(18., cy - 19., w - 36., 16.),
            if state.active_tool == crate::tools::ToolKind::Select {
                "Middle-drag to pan · Scroll to zoom · Space to fit"
            } else {
                "Middle-drag to pan · Scroll to zoom"
            },
            10.,
            ui::muted(),
        );
        if state.size_kind() == crate::editing::SizeKind::None {
            c.centered_in(Rect::new(18., cy, w - 36., 30.), label, 12., ui::muted());
        } else {
            c.centered_in(Rect::new(cx, cy, 60., 30.), label, 12., ui::muted());
        }
        if state.size_kind() != crate::editing::SizeKind::None {
            c.button(
                Rect::new(cx + 66., cy, 28., 30.),
                "−",
                None,
                false,
                self.header.hovered == Some(Action::SizeDown),
                true,
            );
            let value = self
                .size_draft
                .clone()
                .unwrap_or_else(|| format!("{} px", state.tool_size()));
            c.button(
                Rect::new(cx + 100., cy, 68., 30.),
                &value,
                None,
                false,
                self.size_draft.is_some() || self.header.hovered == Some(Action::EditSize),
                true,
            );
            c.button(
                Rect::new(cx + 321., cy, 28., 30.),
                "+",
                None,
                false,
                self.header.hovered == Some(Action::SizeUp),
                true,
            );
            let r = Self::size_slider(w, h, state);
            let (lo, hi) = state.size_kind().limits();
            let f = (state.tool_size() - lo) / (hi - lo);
            c.round(Rect::new(r.x, r.y + 14., r.w, 3.), 1.5, ui::muted(), 0.28);
            c.round(
                Rect::new(r.x, r.y + 14., r.w * f, 3.),
                1.5,
                ui::accent(),
                1.,
            );
            c.round(
                Rect::new(r.x + r.w * f - 5., r.y + 10., 10., 10.),
                5.,
                ui::accent(),
                1.,
            );
        }
        let tx = (w - 612.) / 2.;
        let ty = h - 90.;
        c.round(
            Rect::new(tx, ty, 612., 56.),
            10.,
            crate::theme::bg_3(),
            0.62,
        );
        // Meaningful names stay visible; hover/focus adds shortcuts and consequences.
        for (x, label) in [
            (25., "Undo"),
            (62., "Redo"),
            (466., "Color"),
            (522., "Tools"),
            (578., "Zoom"),
        ] {
            c.centered_in(
                Rect::new(tx + x - 24., ty + 36., 48., 16.),
                label,
                10.5,
                ui::muted(),
            );
        }
        // Undo and Redo match the tool icons: 18 pixels, centred on their targets.
        c.icon(
            "undo",
            tx + 16.,
            ty + 14.,
            18.,
            if state.undo_stack.is_empty() {
                (143, 137, 153)
            } else {
                ui::muted()
            },
        );
        c.icon(
            "redo",
            tx + 53.,
            ty + 14.,
            18.,
            if state.redo_stack.is_empty() {
                (143, 137, 153)
            } else {
                ui::muted()
            },
        );
        for (n, i) in PRIMARY.iter().enumerate() {
            if !state.visible_buttons.contains(i) {
                continue;
            }
            let x = tx + 103. + n as f32 * 45.;
            c.centered_in(
                Rect::new(x - 5.5, ty + 36., 45., 16.),
                tool_label(*i),
                10.5,
                ui::muted(),
            );
            let selected = BUTTONS[*i].action == ButtonAction::Tool(state.active_tool);
            if selected {
                c.round(Rect::new(x, ty + 9., 34., 28.), 7., ui::focus_color(), 0.26);
                if !(self.header.keyboard_focus && self.header.hovered == Some(Action::Button(*i)))
                {
                    c.outline(Rect::new(x, ty + 9., 34., 28.), 7., ui::focus_color(), 1.25);
                }
            } else if self.header.hovered == Some(Action::Button(*i)) {
                c.round(Rect::new(x, ty + 7., 34., 32.), 7., (255, 255, 255), 0.09);
            }
            if let Some(icon) = BUTTONS[*i].icon {
                c.icon(
                    icon,
                    x + 8.,
                    ty + 14.,
                    18.,
                    if selected {
                        ui::focus_color()
                    } else {
                        ui::ink()
                    },
                );
            }
        }
        let rgb = state.current_color;
        c.round(
            Rect::new(tx + 450., ty + 15., 16., 16.),
            8.,
            (
                (rgb.r * 255.) as u8,
                (rgb.g * 255.) as u8,
                (rgb.b * 255.) as u8,
            ),
            1.,
        );
        c.icon("down", tx + 472., ty + 15.5, 15., ui::muted());
        c.icon("more", tx + 513., ty + 14., 18., ui::ink());
        // Zoom text shares the icons' middle line instead of sitting below it.
        c.centered_in(
            Rect::new(tx + 552., ty + 7., 52., 32.),
            &format!("{}%", (zoom * 100.).round() as u32),
            11.,
            ui::muted(),
        );
        state.clear_expired_toast();
        let status = state
            .toast_message
            .as_deref()
            .unwrap_or(&self.header.status);
        let failed = status.contains("failed");
        if !status.trim().is_empty() {
            c.icon(
                if failed { "more" } else { "check" },
                22.,
                h - 23.,
                12.,
                if failed {
                    (232, 162, 147)
                } else {
                    (154, 200, 172)
                },
            );
            c.label(
                42.,
                h - 25.,
                &status
                    .chars()
                    .take(((w - 190.) / 6.).max(8.) as usize)
                    .collect::<String>(),
                11.,
                ui::muted(),
            );
        }
        let bounds = state.editor_canvas_bounds();
        let size = format!("{:.0} x {:.0}", bounds.width, bounds.height);
        let tw = ui::measure(&size, 11. * dpi) / dpi;
        c.label(w - 22. - tw, h - 25., &size, 11., ui::muted());
        if self.header.menu == Menu::Annotation {
            let targets = self.targets(w, h, state);
            let first = targets[0].rect;
            c.round(
                Rect::new(
                    first.x - 4.,
                    first.y - 4.,
                    first.w + 8.,
                    targets.len() as f32 * 32. + 8.,
                ),
                8.,
                ui::panel(),
                0.98,
            );
            for t in targets {
                if self.header.hovered == Some(t.action) {
                    c.round(t.rect, 4., ui::muted(), 0.1);
                }
                let label = match t.action {
                    Action::DeleteAnnotation => "Delete annotation",
                    _ => "",
                };
                c.label(t.rect.x + 12., t.rect.y + 7., label, 13., ui::ink());
            }
        } else if self.header.menu == Menu::Tools || self.header.menu == Menu::Colors {
            let items = self.targets(w, h, state);
            if let Some(first) = items.first() {
                let width = if self.header.menu == Menu::Tools {
                    360.
                } else {
                    180.
                };
                let top = first.rect.y
                    - if self.header.menu == Menu::Tools {
                        24.
                    } else {
                        4.
                    };
                let bottom = items
                    .iter()
                    .map(|t| t.rect.y + t.rect.h)
                    .fold(top, f32::max);
                c.round(
                    Rect::new(first.rect.x - 4., top, width + 8., bottom + 4. - top),
                    10.,
                    ui::panel(),
                    0.99,
                );
                if self.header.menu == Menu::Tools {
                    for (label, annotation) in
                        [("More annotations", true), ("Image actions", false)]
                    {
                        if let Some(t) = items.iter().find(
                            |t| matches!(t.action, Action::Button(i) if (i < 14) == annotation),
                        ) {
                            c.label(t.rect.x + 10., t.rect.y - 19., label, 11., ui::muted());
                        }
                    }
                }
                for t in items {
                    if self.header.hovered == Some(t.action) {
                        c.round(t.rect, 5., (255, 255, 255), 0.09);
                    }
                    if let Action::Button(i) = t.action {
                        let def = &BUTTONS[i];
                        if let Some(icon) = def.icon {
                            c.icon(icon, t.rect.x + 10., t.rect.y + 8., 16., ui::muted());
                            c.label(t.rect.x + 36., t.rect.y + 7., tool_label(i), 12., ui::ink());
                        } else if let ButtonAction::Color(index) = def.action {
                            let color = crate::geometry::Color::presets()[index];
                            c.round(
                                Rect::new(t.rect.x + 12., t.rect.y + 10., 12., 12.),
                                6.,
                                (
                                    (color.r * 255.) as u8,
                                    (color.g * 255.) as u8,
                                    (color.b * 255.) as u8,
                                ),
                                1.,
                            );
                            c.label(
                                t.rect.x + 36.,
                                t.rect.y + 7.,
                                def.tooltip.split(" (").next().unwrap_or(def.tooltip),
                                12.,
                                ui::ink(),
                            );
                        }
                    } else if t.action == Action::CustomColor {
                        c.label(
                            t.rect.x + 10.,
                            t.rect.y + 7.,
                            "Custom color...",
                            12.,
                            ui::ink(),
                        );
                    }
                }
            }
        } else {
            self.header.render_menu(&mut c, w);
            self.header.render_hint(&mut c, w, true);
            if self.header.menu == Menu::None {
                if let Some(t) = self
                    .targets(w, h, state)
                    .iter()
                    .find(|t| t.rect.y > 62. && Some(t.action) == self.header.hovered)
                {
                    let label = match t.action {
                        Action::EditSize => "Type a size; press Enter to apply",
                        Action::SizeDown => "Make the annotation smaller",
                        Action::SizeUp => "Make the annotation larger",
                        Action::SetSize(_) => "Drag to change the annotation size",
                        Action::Button(i) if i < 14 => &self.tool_hints[i],
                        Action::Button(i) => BUTTONS[i].tooltip,
                        Action::Undo => "Undo the last annotation (Ctrl+Z)",
                        Action::Redo => "Redo the last annotation (Ctrl+Shift+Z)",
                        Action::ToggleColors => "Choose the annotation color",
                        Action::ToggleTools => "More annotations, extract text, and pin",
                        Action::ResetZoom => "Reset zoom to fit the window (Ctrl+0)",
                        _ => "",
                    };
                    if !label.is_empty() {
                        c.hint(t.rect, label, true);
                    }
                }
            }
        }
        self.header.render_upload_feedback(&mut c, w, true);
        self.header.render_focus(&mut c, &self.targets(w, h, state));
    }
}
