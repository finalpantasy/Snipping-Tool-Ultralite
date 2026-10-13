#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::{Duration, Instant};

use clap::Parser;
use tray_icon::menu::MenuEvent;
use tray_icon::{MouseButton, MouseButtonState, TrayIconEvent};
use winit::event::{ElementState, MouseButton as WinitMouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy};
use winit::keyboard::{Key, ModifiersState, NamedKey};
#[cfg(not(target_os = "windows"))]
use winit::window::WindowLevel;
use winit::window::{CursorIcon, Window, WindowAttributes, WindowId};

use winit::application::ApplicationHandler;

use notify_rust::Notification;

use snipping_tool_ul::capture;
use snipping_tool_ul::cli::{Cli, Commands};
use snipping_tool_ul::config::Config;
use snipping_tool_ul::countdown::Countdown;
use snipping_tool_ul::export;
use snipping_tool_ul::geometry::{Color, Point};
use snipping_tool_ul::history_ui::HistoryWindow;
use snipping_tool_ul::launcher::{
    Action as HomeAction, CaptureMode, LauncherWindow, Model as HomeModel,
};
use snipping_tool_ul::overlay::selection::{HitZone, Selection};
use snipping_tool_ul::overlay::toolbar::{ButtonAction, BUTTONS};
use snipping_tool_ul::pin::PinnedWindow;
use snipping_tool_ul::renderer::render_overlay;
use snipping_tool_ul::settings_ui::SettingsWindow;
use snipping_tool_ul::state::{AppState, OverlayState};
use snipping_tool_ul::tools::{
    annotation_bounding_box, apply_redo, apply_undo, hit_test_annotation, move_annotation,
    recolor_annotation, record_undo, resize_annotation, Annotation, AnnotationTool, ResizeHandle,
    ToolKind, UndoAction,
};
use snipping_tool_ul::tray::{self, TrayState};
use snipping_tool_ul::window_detect;

struct SnipResult {
    screen: capture::CapturedScreen,
    clipboard_error: Option<String>,
}

enum UserEvent {
    IntegrationReady(Result<String, String>),
    ShowHome,
    Hotkey(global_hotkey::GlobalHotKeyEvent),
    Tray(TrayIconEvent),
    Menu(MenuEvent),
    Status {
        revision: u64,
        message: String,
        upload_done: bool,
    },
    SnipReady(Result<Option<SnipResult>, String>),
    SavePathReady {
        revision: u64,
        path: Option<std::path::PathBuf>,
    },
    UploadFinished {
        revision: u64,
        key: [u8; 32],
        url: Option<String>,
        message: String,
    },
}

struct App {
    pending_menu: Vec<MenuEvent>,
    pending_tray: Vec<TrayIconEvent>,
    launcher: Option<LauncherWindow>,
    show_home_on_start: bool,
    capture_mode: CaptureMode,
    delay_seconds: u32,
    editor_view: snipping_tool_ul::preview::EditorView,
    editor_ui: snipping_tool_ul::editor_ui::EditorUi,
    native_surface: Option<snipping_tool_ul::native_surface::Surface>,
    preview_frame: Option<tiny_skia::Pixmap>,
    preview_renderer: snipping_tool_ul::preview_render::PreviewRenderer,
    restore_home_on_cancel: bool,
    editor_pointer_down: bool,
    editor_pointer: (f64, f64),
    image_revision: u64,
    upload_pending: bool,
    upload_session: snipping_tool_ul::upload::UploadSession,
    save_dialog_pending: bool,
    integration_pending: bool,
    integration_status: String,
    proxy: EventLoopProxy<UserEvent>,
    picker_active: bool,
    is_preview: bool,
    config: Config,
    state: AppState,
    tray: Option<TrayState>,
    overlay_window: Option<Arc<Window>>,
    surface: Option<softbuffer::Surface<Arc<Window>, Arc<Window>>>,
    pixmap: Option<tiny_skia::Pixmap>,
    modifiers: ModifiersState,
    /// Modifier state tracked separately for the settings window (used while
    /// capturing a new global-hotkey combination).
    settings_modifiers: ModifiersState,
    needs_redraw: bool,
    last_render: Instant,
    capture_at: Option<Instant>,
    _hotkey_manager: Option<global_hotkey::GlobalHotKeyManager>,
    hotkey_id: Option<u32>,
    /// Set once the working set has been trimmed for the current tray-idle period.
    idle_trimmed: bool,
    pinned_windows: Vec<PinnedWindow>,
    immediate_capture: bool,
    cli_only: bool,
    countdown: Option<Countdown>,
    window_capture_mode: bool,
    window_rects: Vec<window_detect::WinRect>,
    settings_window: Option<SettingsWindow>,
    history_window: Option<HistoryWindow>,
}

const FRAME_INTERVAL: Duration = Duration::from_millis(8); // event-driven, up to 125fps while editing

impl App {
    fn new(config: Config, proxy: EventLoopProxy<UserEvent>) -> Self {
        snipping_tool_ul::theme::set_mode(config.theme_mode());
        let editor_ui = snipping_tool_ul::editor_ui::EditorUi::new(&config);
        Self {
            pending_menu: Vec::new(),
            pending_tray: Vec::new(),
            launcher: None,
            show_home_on_start: false,
            capture_mode: CaptureMode::Region,
            delay_seconds: 0,
            editor_view: Default::default(),
            editor_ui,
            native_surface: None,
            preview_frame: None,
            preview_renderer: Default::default(),
            restore_home_on_cancel: false,
            editor_pointer_down: false,
            editor_pointer: (0., 0.),
            image_revision: 0,
            upload_pending: false,
            upload_session: Default::default(),
            save_dialog_pending: false,
            integration_pending: false,
            integration_status: String::new(),
            proxy,
            picker_active: false,
            is_preview: false,
            config,
            state: AppState::Idle,
            tray: None,
            overlay_window: None,
            surface: None,
            pixmap: None,
            modifiers: ModifiersState::empty(),
            settings_modifiers: ModifiersState::empty(),
            needs_redraw: false,
            last_render: Instant::now(),
            capture_at: None,
            _hotkey_manager: None,
            hotkey_id: None,
            idle_trimmed: false,
            pinned_windows: Vec::new(),
            immediate_capture: false,
            cli_only: false,
            countdown: None,
            window_capture_mode: false,
            window_rects: Vec::new(),
            settings_window: None,
            history_window: None,
        }
    }

    fn show_home(&mut self, event_loop: &ActiveEventLoop) {
        if self.picker_active {
            return;
        }
        if self.launcher.is_none() {
            let mut model = HomeModel::new(
                self.config.workflow.clone(),
                self.config.hotkey.capture.clone(),
            );
            model.mode = self.capture_mode;
            model.delay = self.delay_seconds;
            match LauncherWindow::create(event_loop, model) {
                Ok(home) => self.launcher = Some(home),
                Err(e) => {
                    tracing::error!("Home window: {e}");
                    return;
                }
            }
        }
        if let Some(home) = &mut self.launcher {
            home.model.has_image = matches!(self.state, AppState::Capturing(_));
            home.model.connected = !self.config.general.imgur_client_id.is_empty()
                || std::env::var("SNIPPING_TOOL_UL_IMGUR_CLIENT_ID").is_ok_and(|v| !v.is_empty());
            home.model.workflow = self.config.workflow.clone();
            home.model.hotkey = self.config.hotkey.capture.clone();
            let _ = home.render();
            home.window.set_visible(true);
            home.window.focus_window();
        }
    }

    fn home_action(&mut self, event_loop: &ActiveEventLoop, action: HomeAction) {
        if matches!(
            action,
            HomeAction::Button(_)
                | HomeAction::Upload
                | HomeAction::Undo
                | HomeAction::Redo
                | HomeAction::CustomColor
        ) {
            if let AppState::Capturing(ref mut o) = self.state {
                o.commit_text();
            }
        }
        match action {
            HomeAction::Quit => {
                event_loop.exit();
                return;
            }
            HomeAction::SizeDown | HomeAction::SizeUp | HomeAction::SetSize(_) => {
                if let AppState::Capturing(ref mut o) = self.state {
                    let step = if o.size_kind() == snipping_tool_ul::editing::SizeKind::Width {
                        0.5
                    } else {
                        1.
                    };
                    let value = match action {
                        HomeAction::SizeDown => o.tool_size() - step,
                        HomeAction::SizeUp => o.tool_size() + step,
                        HomeAction::SetSize(v) => v as f32 / 10.,
                        _ => unreachable!(),
                    };
                    o.set_tool_size(value, !self.editor_ui.size_dragging);
                    self.needs_redraw = true;
                }
            }
            HomeAction::EditSize => {
                if let AppState::Capturing(ref o) = self.state {
                    self.editor_ui.size_draft = Some(o.tool_size().to_string());
                    self.editor_ui.size_select_all = true;
                    self.needs_redraw = true;
                }
            }
            HomeAction::DeleteAnnotation => {
                if let AppState::Capturing(ref mut o) = self.state {
                    if let Some(i) = o.selected_index {
                        o.delete_annotation(i);
                        self.needs_redraw = true;
                    }
                }
            }
            HomeAction::New => {
                self.restore_home_on_cancel = true;
                if self.picker_active || self.countdown.is_some() || self.capture_at.is_some() {
                    return;
                }
                if let Some(home) = &self.launcher {
                    home.window.set_visible(false);
                }
                if self.delay_seconds == 0 {
                    self.trigger_capture(event_loop);
                } else {
                    self.close_overlay();
                    self.countdown = Countdown::start(event_loop, self.delay_seconds);
                    if self.countdown.is_none() {
                        self.trigger_capture(event_loop);
                    }
                }
            }
            HomeAction::Mode(mode) => {
                self.capture_mode = mode;
                if let Some(h) = &mut self.launcher {
                    h.model.mode = mode;
                }
            }
            HomeAction::SetDelay(seconds) => {
                self.delay_seconds = seconds;
                if let Some(h) = &mut self.launcher {
                    h.model.delay = seconds;
                }
            }
            HomeAction::Delay => {
                if let Some(h) = &mut self.launcher {
                    h.model.cycle_delay();
                    self.delay_seconds = h.model.delay;
                }
            }
            HomeAction::Preview => {
                self.config.workflow.open_preview = !self.config.workflow.open_preview
            }
            HomeAction::AutoSave => {
                self.config.workflow.auto_save = !self.config.workflow.auto_save
            }
            HomeAction::ConfirmUpload => {
                self.config.workflow.confirm_upload = !self.config.workflow.confirm_upload
            }
            HomeAction::Settings => self.open_settings(event_loop),
            HomeAction::History => self.open_history(event_loop),
            HomeAction::Upload => self.do_upload(event_loop, false),
            HomeAction::ConfirmPublicUpload => self.do_upload(event_loop, true),
            HomeAction::CancelUpload => {
                self.upload_session.cancel();
                if let Some(home) = &mut self.launcher {
                    home.model.menu = snipping_tool_ul::launcher::Menu::None;
                }
            }
            HomeAction::Connect => {
                self.editor_ui.header.menu = snipping_tool_ul::launcher::Menu::Hosts;
                if let Some(h) = &mut self.launcher {
                    h.model.menu = snipping_tool_ul::launcher::Menu::Hosts;
                }
            }
            HomeAction::SetHost(host) => {
                self.config.workflow.upload_host = host;
                self.upload_session.cancel();
                self.editor_ui.header.menu = snipping_tool_ul::launcher::Menu::Options;
                if let Some(h) = &mut self.launcher {
                    h.model.menu = snipping_tool_ul::launcher::Menu::Options;
                }
            }
            HomeAction::Button(i) => self.editor_button(event_loop, i),
            HomeAction::Undo | HomeAction::Redo => {
                if let AppState::Capturing(ref mut o) = self.state {
                    if action == HomeAction::Undo {
                        apply_undo(&mut o.annotations, &mut o.undo_stack, &mut o.redo_stack);
                    } else {
                        apply_redo(&mut o.annotations, &mut o.undo_stack, &mut o.redo_stack);
                    }
                    o.selected_index = None;
                    o.sync_step_counter();
                    self.needs_redraw = true;
                }
            }
            HomeAction::ResetZoom => {
                self.editor_view = Default::default();
                self.needs_redraw = true;
            }
            HomeAction::CustomColor => {
                if let AppState::Capturing(ref mut o) = self.state {
                    if let Some(color) =
                        snipping_tool_ul::color_picker::pick_color(&o.current_color)
                    {
                        if let Some(n) = o.selected_index {
                            if n < o.annotations.len() {
                                let old = o.annotations[n].clone();
                                recolor_annotation(&mut o.annotations[n], color);
                                record_undo(
                                    &mut o.undo_stack,
                                    &mut o.redo_stack,
                                    UndoAction::Modify(n, old),
                                );
                            }
                        } else {
                            o.set_color_all(color);
                        }
                        self.needs_redraw = true;
                    }
                }
            }
            HomeAction::ToggleModes
            | HomeAction::ToggleDelay
            | HomeAction::ToggleOptions
            | HomeAction::ToggleTools
            | HomeAction::ToggleColors
            | HomeAction::Dismiss => {}
            HomeAction::CancelConnection => {
                if let Some(h) = &mut self.launcher {
                    h.model.connecting = false;
                    h.model.draft.clear();
                }
            }
            HomeAction::SaveConnection => {
                if let Some(h) = &mut self.launcher {
                    self.config.general.imgur_client_id = h.model.draft.clone();
                    h.model.connected = !h.model.draft.is_empty();
                    h.model.connecting = false;
                    h.model.draft.clear();
                }
            }
        }
        if matches!(
            action,
            HomeAction::Preview
                | HomeAction::AutoSave
                | HomeAction::ConfirmUpload
                | HomeAction::SaveConnection
                | HomeAction::SetHost(_)
        ) {
            if let Err(e) = self.config.save() {
                if let Some(h) = &mut self.launcher {
                    h.model.status = format!("Settings could not be saved: {e}");
                }
            }
        }
        self.editor_ui.header.mode = self.capture_mode;
        self.editor_ui.header.delay = self.delay_seconds;
        self.editor_ui.sync_preferences(&self.config);
        if !matches!(
            action,
            HomeAction::Preview
                | HomeAction::AutoSave
                | HomeAction::ConfirmUpload
                | HomeAction::Upload
                | HomeAction::ConfirmPublicUpload
                | HomeAction::Connect
                | HomeAction::SetHost(_)
        ) {
            self.editor_ui.header.menu = snipping_tool_ul::launcher::Menu::None;
        }
        if self.is_preview {
            self.needs_redraw = true;
        }
        if let Some(h) = &mut self.launcher {
            h.model.workflow = self.config.workflow.clone();
            h.window.request_redraw();
        }
    }

    fn editor_button(&mut self, event_loop: &ActiveEventLoop, index: usize) {
        let Some(action) = BUTTONS.get(index).map(|d| d.action) else {
            return;
        };
        if let AppState::Capturing(ref mut o) = self.state {
            match action {
                ButtonAction::Tool(kind) => {
                    o.active_tool = kind;
                    if kind != ToolKind::Select {
                        o.selected_index = None;
                    }
                }
                ButtonAction::Color(i) => {
                    if let Some(color) = Color::presets().get(i).copied() {
                        if let Some(n) = o.selected_index {
                            if n < o.annotations.len() {
                                let old = o.annotations[n].clone();
                                recolor_annotation(&mut o.annotations[n], color);
                                record_undo(
                                    &mut o.undo_stack,
                                    &mut o.redo_stack,
                                    UndoAction::Modify(n, old),
                                );
                            }
                        } else {
                            o.set_color_all(color);
                        }
                    }
                }
                _ => {}
            }
        }
        self.needs_redraw = true;
        match action {
            ButtonAction::Copy => self.do_copy(),
            ButtonAction::Save => self.do_save(),
            ButtonAction::Upload => self.do_upload(event_loop, false),
            ButtonAction::Ocr => self.do_ocr(),
            ButtonAction::Pin => self.do_pin(event_loop),
            _ => {}
        }
    }

    /// Register the global capture hotkey from config, falling back to the
    /// default binding when the configured one fails to parse or register.
    fn register_hotkey_with_fallback(&mut self) {
        // Drop any previous registration first.
        self._hotkey_manager = None;
        self.hotkey_id = None;

        let binding = self.config.hotkey.capture.clone();
        match snipping_tool_ul::hotkey::register_hotkey(&binding) {
            Ok((manager, id)) => {
                self._hotkey_manager = Some(manager);
                self.hotkey_id = Some(id);
                tracing::info!("Global hotkey registered: {}", binding);
            }
            Err(e) => {
                tracing::warn!("Failed to register hotkey '{}': {}", binding, e);
                const DEFAULT: &str = "Ctrl+Shift+S";
                if binding != DEFAULT {
                    match snipping_tool_ul::hotkey::register_hotkey(DEFAULT) {
                        Ok((manager, id)) => {
                            self._hotkey_manager = Some(manager);
                            self.hotkey_id = Some(id);
                            tracing::info!("Fell back to default hotkey {DEFAULT}");
                            let _ = Notification::new()
                                .summary("Snipping Tool UL")
                                .body(&format!(
                                    "Hotkey '{binding}' could not be registered — using {DEFAULT}"
                                ))
                                .timeout(4000)
                                .show();
                        }
                        Err(e2) => tracing::warn!("Default hotkey also failed: {e2}"),
                    }
                }
            }
        }
    }

    fn trigger_capture(&mut self, _event_loop: &ActiveEventLoop) {
        #[cfg(target_os = "windows")]
        {
            if self.picker_active {
                return;
            }
            // Previous previews are closed before selecting so they cannot be
            // accidentally included in the next screenshot.
            self.restore_home_on_cancel |= self
                .launcher
                .as_ref()
                .is_some_and(|h| h.window.is_visible().unwrap_or(false));
            self.launcher = None; // Release home render resources before the next capture.
            self.countdown = None;
            self.capture_at = None;
            let mode = self.capture_mode;
            self.close_overlay();
            self.image_revision = self.image_revision.wrapping_add(1);
            let clipboard_ticket = snipping_tool_ul::clipboard_queue::reserve();
            self.picker_active = true;
            let proxy = self.proxy.clone();
            std::thread::spawn(move || {
                let selected = match mode {
                    CaptureMode::Region => snipping_tool_ul::region_picker::select_region(),
                    CaptureMode::Window => snipping_tool_ul::region_picker::select_window(),
                    CaptureMode::FullScreen => {
                        unsafe {
                            let _ = windows::Win32::Graphics::Dwm::DwmFlush();
                        }
                        Ok(Some(capture::virtual_bounds()))
                    }
                };
                let result = selected.and_then(|rect| {
                    rect.map(|rect| {
                        let screen = capture::capture_region(rect).map_err(|e| e.to_string())?;
                        // Populate the real Windows clipboard before displaying
                        // the preview. No extra user click and no full desktop copy.
                        let clipboard_error = match export::copy_with_ticket(
                            clipboard_ticket,
                            &screen.pixels,
                            screen.width,
                            screen.height,
                        ) {
                            Ok(true) => None,
                            Ok(false) => Some(
                                "A newer clipboard request was kept. Use Copy to copy this image."
                                    .into(),
                            ),
                            Err(error) => Some(error),
                        };
                        Ok(SnipResult {
                            screen,
                            clipboard_error,
                        })
                    })
                    .transpose()
                });
                let _ = proxy.send_event(UserEvent::SnipReady(result));
            });
            return;
        }
        #[cfg(not(target_os = "windows"))]
        {
            let event_loop = _event_loop;
            if self.overlay_window.is_some() {
                tracing::info!("Overlay already open, ignoring capture trigger");
                return;
            }

            let capturer = match capture::create_capturer() {
                Ok(c) => c,
                Err(e) => {
                    tracing::error!("Failed to create capturer: {e}");
                    return;
                }
            };

            let screens = match capturer.capture_all_screens() {
                Ok(s) => s,
                Err(e) => {
                    tracing::error!("Screen capture failed: {e}");
                    return;
                }
            };

            if screens.is_empty() {
                tracing::error!("No screens captured");
                return;
            }

            // Use the first (primary) screen
            let screenshot = screens.into_iter().next().unwrap();
            tracing::info!(
                "Captured screen: {}x{} at ({}, {})",
                screenshot.width,
                screenshot.height,
                screenshot.x_offset,
                screenshot.y_offset
            );

            let attrs = WindowAttributes::default()
                .with_position(winit::dpi::PhysicalPosition::new(
                    screenshot.x_offset,
                    screenshot.y_offset,
                ))
                .with_inner_size(winit::dpi::PhysicalSize::new(
                    screenshot.width,
                    screenshot.height,
                ))
                .with_decorations(false)
                .with_visible(false)
                .with_window_level(WindowLevel::AlwaysOnTop)
                .with_title("Snipping Tool UL");

            let window = match event_loop.create_window(attrs) {
                Ok(w) => Arc::new(w),
                Err(e) => {
                    tracing::error!("Failed to create overlay window: {e}");
                    return;
                }
            };

            let context = match softbuffer::Context::new(Arc::clone(&window)) {
                Ok(c) => c,
                Err(e) => {
                    tracing::error!("Failed to create softbuffer context: {e}");
                    return;
                }
            };

            let surface = match softbuffer::Surface::new(&context, Arc::clone(&window)) {
                Ok(s) => s,
                Err(e) => {
                    tracing::error!("Failed to create softbuffer surface: {e}");
                    return;
                }
            };

            let pixmap = tiny_skia::Pixmap::new(screenshot.width, screenshot.height);
            if pixmap.is_none() {
                tracing::error!(
                    "Failed to create pixmap ({}x{})",
                    screenshot.width,
                    screenshot.height
                );
                return;
            }

            self.state = AppState::Capturing(Box::new(OverlayState::new(screenshot, &self.config)));
            self.surface = Some(surface);
            self.pixmap = pixmap;
            self.overlay_window = Some(window);
            self.needs_redraw = true;

            if let Some(w) = &self.overlay_window {
                w.request_redraw();
            }
        }
    }

    fn open_preview(
        &mut self,
        event_loop: &ActiveEventLoop,
        screenshot: capture::CapturedScreen,
        copied: bool,
    ) -> Result<(), String> {
        self.close_overlay();
        let height = screenshot.height;
        let attrs = WindowAttributes::default()
            .with_title("Snipping Tool UL")
            .with_inner_size(winit::dpi::LogicalSize::new(940, 612))
            .with_min_inner_size(winit::dpi::LogicalSize::new(
                snipping_tool_ul::editor_ui::MIN_WIDTH,
                snipping_tool_ul::editor_ui::MIN_HEIGHT,
            ))
            .with_visible(false)
            .with_decorations(true)
            .with_resizable(true);
        let setup = (|| {
            let window = Arc::new(event_loop.create_window(attrs).map_err(|e| e.to_string())?);
            let surface = snipping_tool_ul::native_surface::Surface::new(window.clone())?;
            Ok::<_, String>((window, surface))
        })();
        match setup {
            Ok((window, surface)) => {
                let selection = Selection {
                    x: 0.0,
                    y: 0.0,
                    width: screenshot.width as f32,
                    height: screenshot.height as f32,
                };
                if self.config.general.history_enabled {
                    let pixels = screenshot.pixels.clone();
                    let (w, h) = (screenshot.width, screenshot.height);
                    std::thread::spawn(move || {
                        let _ = snipping_tool_ul::history::save_to_history(&pixels, w, h);
                    });
                }
                let mut state = OverlayState::new(screenshot, &self.config);
                state.canvas_height = height;
                state.preview_mode = true;
                state.active_tool = ToolKind::Pencil;
                self.editor_view = Default::default();
                state.selection = Some(selection);
                self.state = AppState::Capturing(Box::new(state));
                self.overlay_window = Some(window);
                self.native_surface = Some(surface);
                self.surface = None;
                self.preview_frame = None;
                self.editor_ui = snipping_tool_ul::editor_ui::EditorUi::new(&self.config);
                self.editor_ui.header.mode = self.capture_mode;
                self.editor_ui.header.delay = self.delay_seconds;
                self.editor_ui.header.status = if copied {
                    "Copied to clipboard"
                } else {
                    "Copy failed. Click Copy to retry."
                }
                .into();
                if !copied {
                    if let AppState::Capturing(ref mut o) = self.state {
                        o.show_toast("Copy failed. Click Copy to retry.".into(), 6000);
                    }
                }
                self.pixmap = None;
                self.preview_renderer = Default::default();
                self.is_preview = true;
                self.needs_redraw = true;
                self.render();
                Ok(())
            }
            Err(error) => Err(error),
        }
    }

    fn open_settings(&mut self, event_loop: &ActiveEventLoop) {
        if self.settings_window.is_some() {
            tracing::info!("Settings window already open");
            return;
        }

        let config = Config::load();

        let win_icon = snipping_tool_ul::launcher::window_icon();

        let mut attrs = WindowAttributes::default()
            .with_title("Snipping Tool UL Settings")
            .with_inner_size(winit::dpi::LogicalSize::new(
                snipping_tool_ul::settings_ui::WIN_W,
                snipping_tool_ul::settings_ui::WIN_H,
            ))
            .with_resizable(false)
            .with_decorations(true);

        if let Some(icon) = win_icon {
            attrs = attrs.with_window_icon(Some(icon));
        }

        let window = match event_loop.create_window(attrs) {
            Ok(w) => Arc::new(w),
            Err(e) => {
                tracing::error!("Failed to create settings window: {e}");
                return;
            }
        };

        snipping_tool_ul::launcher::style_window(&window);
        let surface = match snipping_tool_ul::native_surface::Surface::new(window.clone()) {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("Failed to create settings surface: {e}");
                return;
            }
        };

        let mut sw = SettingsWindow::new(window, surface, config);
        sw.integration_pending = self.integration_pending;
        sw.integration_status = self.integration_status.clone();
        sw.render();
        tracing::info!("Settings window opened");
        self.settings_window = Some(sw);
    }

    fn close_settings(&mut self, apply: bool) {
        if apply {
            let old_binding = self.config.hotkey.capture.clone();
            self.config = Config::load(); // reload saved config
            self.editor_ui.sync_preferences(&self.config);
            snipping_tool_ul::theme::set_mode(self.config.theme_mode());
            if self.config.hotkey.capture != old_binding && !self.cli_only {
                self.register_hotkey_with_fallback();
            }
        } else {
            // Discard any live theme preview from the Settings toggle.
            snipping_tool_ul::theme::set_mode(self.config.theme_mode());
        }
        // Settings can toggle autostart — keep the tray checkbox in sync.
        if let Some(ref tray) = self.tray {
            tray.autostart_check
                .set_checked(snipping_tool_ul::autostart::is_enabled());
        }
        self.settings_window = None;
        if let Some(home) = &mut self.launcher {
            home.model.workflow = self.config.workflow.clone();
            home.model.hotkey = self.config.hotkey.capture.clone();
            snipping_tool_ul::launcher::style_window(&home.window);
            home.window.request_redraw();
        }
        if let AppState::Capturing(ref mut o) = self.state {
            o.visible_buttons = self.config.toolbar.visible_button_indices();
        }
        if let Some(window) = &self.overlay_window {
            if self.is_preview {
                snipping_tool_ul::launcher::style_window(window);
                self.needs_redraw = true;
                window.request_redraw();
            }
        }
        tracing::info!("Settings window closed");
    }

    fn open_history(&mut self, event_loop: &ActiveEventLoop) {
        if self.history_window.is_some() {
            tracing::info!("History window already open");
            return;
        }

        let win_icon = snipping_tool_ul::launcher::window_icon();

        let mut attrs = WindowAttributes::default()
            .with_title("Snipping Tool UL \u{2014} History")
            .with_inner_size(winit::dpi::LogicalSize::new(
                snipping_tool_ul::history_ui::WIN_W,
                snipping_tool_ul::history_ui::WIN_H,
            ))
            .with_resizable(false)
            .with_decorations(true);

        if let Some(icon) = win_icon {
            attrs = attrs.with_window_icon(Some(icon));
        }

        let window = match event_loop.create_window(attrs) {
            Ok(w) => Arc::new(w),
            Err(e) => {
                tracing::error!("Failed to create history window: {e}");
                return;
            }
        };

        let context = match softbuffer::Context::new(Arc::clone(&window)) {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("Failed to create history context: {e}");
                return;
            }
        };

        let surface = match softbuffer::Surface::new(&context, Arc::clone(&window)) {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("Failed to create history surface: {e}");
                return;
            }
        };

        let mut hw = HistoryWindow::new(window, surface);
        snipping_tool_ul::launcher::style_window(&hw.window);
        hw.render();
        tracing::info!("History window opened");
        self.history_window = Some(hw);
    }

    fn close_history(&mut self) {
        self.history_window = None;
        tracing::info!("History window closed");
    }

    fn close_overlay(&mut self) {
        self.is_preview = false;
        self.native_surface = None;
        self.editor_pointer_down = false;
        self.editor_view.end_pan();
        self.preview_frame = None;
        self.preview_renderer = Default::default();
        self.surface = None;
        self.pixmap = None;
        self.overlay_window = None;
        self.state = AppState::Idle;
        self.modifiers = ModifiersState::empty();
        self.window_capture_mode = false;
        self.window_rects.clear();
        // A redraw flagged just before closing (Pin does this) has no window
        // left to clear it, which would keep about_to_wait waking every frame.
        self.needs_redraw = false;
        tracing::info!("Overlay closed");
    }

    fn do_copy(&mut self) {
        if let AppState::Capturing(ref mut o) = self.state {
            o.commit_text();
        }
        let data = if let AppState::Capturing(ref o) = self.state {
            o.flattened_selection()
        } else {
            None
        };
        if let Some((pixels, w, h)) = data {
            let ticket = snipping_tool_ul::clipboard_queue::reserve();
            let revision = self.image_revision;
            let proxy = self.proxy.clone();
            let history = self.config.general.history_enabled;
            if !self.is_preview {
                self.close_overlay();
            }
            std::thread::spawn(move || {
                let message = match export::copy_with_ticket(ticket, &pixels, w, h) {
                    Ok(true) => {
                        if history {
                            let _ = snipping_tool_ul::history::save_to_history(&pixels, w, h);
                        }
                        "Copied to clipboard".into()
                    }
                    Ok(false) => return,
                    Err(e) => format!("Copy failed: {e}. Click Copy to retry."),
                };
                let _ = proxy.send_event(UserEvent::Status {
                    revision,
                    message,
                    upload_done: false,
                });
            });
        }
    }

    fn do_save(&mut self) {
        if self.save_dialog_pending {
            return;
        }
        if !matches!(&self.state, AppState::Capturing(o) if o.selection.as_ref().is_some_and(|s| { let r=s.clamped(o.screenshot.width,o.screenshot.height); r.w>0 && r.h>0 }))
        {
            return;
        }
        // Start the chooser before copying pixels, rendering annotations or encoding.
        self.save_dialog_pending = true;
        let dir = self.config.auto_save_directory().ok();
        let revision = self.image_revision;
        let proxy = self.proxy.clone();
        std::thread::spawn(move || {
            let path = export::choose_save_path(dir.as_deref());
            let _ = proxy.send_event(UserEvent::SavePathReady { revision, path });
        });
    }

    fn save_chosen_path(&mut self, path: std::path::PathBuf) {
        let save_data = if let AppState::Capturing(ref mut overlay) = self.state {
            overlay.commit_text();
            overlay.selection.as_ref().and_then(|sel| {
                let r = sel.clamped(overlay.screenshot.width, overlay.screenshot.height);
                overlay.raw_selection().map(|(pixels, w, h)| {
                    (
                        pixels,
                        w,
                        h,
                        overlay
                            .annotations
                            .iter()
                            .map(|a| export::offset_annotation(a, r.x as f32, r.y as f32))
                            .collect::<Vec<_>>(),
                    )
                })
            })
        } else {
            None
        };
        if let Some((pixels, w, h, annotations)) = save_data {
            let history_enabled = self.config.general.history_enabled;
            if !self.is_preview {
                self.close_overlay();
            }
            std::thread::spawn(move || {
                let pixels = if annotations.is_empty() {
                    pixels
                } else {
                    export::flatten_annotations(&pixels, w, h, &annotations)
                };
                match export::save_image_to_path(pixels, w, h, &path) {
                    Ok(()) => {
                        // Copy saved file to history instead of re-encoding
                        if history_enabled {
                            let _ = snipping_tool_ul::history::save_to_history_from_file(&path);
                        }
                        tracing::info!("Saved to {}", path.display());
                        let _ = Notification::new()
                            .summary("Snipping Tool UL")
                            .body(&format!("Saved to {}", path.display()))
                            .timeout(2000)
                            .show();
                    }
                    Err(e) => {
                        tracing::error!("Save failed: {e}");
                        let _ = Notification::new()
                            .summary("Snipping Tool UL")
                            .body(&format!("Save failed: {e}"))
                            .timeout(3000)
                            .show();
                    }
                }
            });
        }
    }

    fn do_upload(&mut self, event_loop: &ActiveEventLoop, explicit_accept: bool) {
        if self.upload_pending {
            return;
        }
        let destination = self.config.workflow.upload_host;
        let host = destination.name();
        let data = if let AppState::Capturing(ref o) = self.state {
            o.flattened_selection()
        } else {
            None
        };
        if let Some((pixels, w, h)) = data {
            let key = snipping_tool_ul::upload::image_key_for_host(&pixels, w, h, destination);
            let cached = match self.upload_session.request_for_host(
                key,
                self.config.workflow.confirm_upload,
                explicit_accept,
                destination,
            ) {
                snipping_tool_ul::upload::UploadDecision::Busy => return,
                snipping_tool_ul::upload::UploadDecision::Cooldown(remaining) => {
                    let seconds = remaining.as_secs_f64().ceil() as u64;
                    let _ = self.proxy.send_event(UserEvent::Status {
                        revision: self.image_revision,
                        message: format!("Please wait {seconds}s before another upload."),
                        upload_done: false,
                    });
                    return;
                }
                snipping_tool_ul::upload::UploadDecision::Confirm => {
                    if !self.is_preview {
                        self.show_home(event_loop);
                    }
                    self.editor_ui.header.menu = snipping_tool_ul::launcher::Menu::UploadConfirm;
                    self.editor_ui.header.connected = host == "Imgur";
                    self.editor_ui.header.hovered = Some(HomeAction::CancelUpload);
                    if let Some(home) = &mut self.launcher {
                        home.model.menu = snipping_tool_ul::launcher::Menu::UploadConfirm;
                        home.model.connected = host == "Imgur";
                        home.model.hovered = Some(HomeAction::CancelUpload);
                        home.window.request_redraw();
                    }
                    self.needs_redraw = true;
                    return;
                }
                snipping_tool_ul::upload::UploadDecision::Reuse(url) => Some(url),
                snipping_tool_ul::upload::UploadDecision::Start => None,
            };
            self.editor_ui.header.menu = snipping_tool_ul::launcher::Menu::None;
            if let Some(home) = &mut self.launcher {
                home.model.menu = snipping_tool_ul::launcher::Menu::None;
            }
            self.upload_pending = cached.is_none();
            let ticket = snipping_tool_ul::clipboard_queue::reserve();
            let revision = self.image_revision;
            let proxy = self.proxy.clone();
            if let AppState::Capturing(ref mut o) = self.state {
                o.show_toast(
                    if cached.is_some() {
                        "Copying existing link...".into()
                    } else {
                        format!("Uploading to {host}...")
                    },
                    60000,
                );
            }
            self.needs_redraw = true;
            std::thread::spawn(move || {
                let reused = cached.is_some();
                let result = cached.map(Ok).unwrap_or_else(|| {
                    encode_png(pixels, w, h).and_then(|png| {
                        snipping_tool_ul::upload::upload_no_setup(&png, destination)
                    })
                });
                let url = result.as_ref().ok().cloned();
                let message = match result {
                    Ok(url) => match snipping_tool_ul::clipboard_queue::run(ticket, || {
                        arboard::Clipboard::new()
                            .map_err(|e| e.to_string())?
                            .set_text(&url)
                            .map_err(|e| e.to_string())
                    }) {
                        Ok(true) => format!(
                            "{}: {url}",
                            if reused {
                                "Existing link copied"
                            } else {
                                "Link copied"
                            }
                        ),
                        Ok(false) => format!("Uploaded: {url} (newer clipboard content kept)"),
                        Err(_) => format!("Uploaded: {url} (link copy failed)"),
                    },
                    Err(e) => format!("Upload failed: {e}. Image kept; click Upload to retry."),
                };
                if reused {
                    let _ = proxy.send_event(UserEvent::Status {
                        revision,
                        message,
                        upload_done: false,
                    });
                } else {
                    let _ = proxy.send_event(UserEvent::UploadFinished {
                        revision,
                        key,
                        url,
                        message,
                    });
                }
            });
        }
    }

    fn do_ocr(&mut self) {
        // OCR reads the raw screenshot pixels (annotations would confuse it)
        let ocr_data = if let AppState::Capturing(ref overlay) = self.state {
            overlay.raw_selection()
        } else {
            None
        };

        if let Some((cropped, w, h)) = ocr_data {
            let ticket = snipping_tool_ul::clipboard_queue::reserve();
            let revision = self.image_revision;
            let proxy = self.proxy.clone();
            if !self.is_preview {
                self.close_overlay();
            }
            std::thread::spawn(move || {
                let toast_msg = match snipping_tool_ul::ocr::extract_text(&cropped, w, h) {
                    Ok(text) => {
                        let char_count = text.chars().count();
                        match snipping_tool_ul::clipboard_queue::run(ticket, || {
                            arboard::Clipboard::new()
                                .map_err(|e| e.to_string())?
                                .set_text(&text)
                                .map_err(|e| e.to_string())
                        }) {
                            Ok(true) => format!("OCR copied {char_count} characters"),
                            Ok(false) => "OCR finished; newer clipboard content kept".into(),
                            Err(e) => format!("OCR clipboard copy failed: {e}"),
                        }
                    }
                    Err(e) => {
                        tracing::error!("OCR failed: {e}");
                        format!("OCR failed: {e}")
                    }
                };
                let _ = proxy.send_event(UserEvent::Status {
                    revision,
                    message: toast_msg,
                    upload_done: false,
                });
            });
        }
    }

    fn do_pin(&mut self, event_loop: &ActiveEventLoop) {
        if let AppState::Capturing(ref overlay) = self.state {
            if let Some((pixels, w, h)) = overlay.flattened_selection() {
                if self.config.general.history_enabled {
                    let _ = snipping_tool_ul::history::save_to_history(&pixels, w, h);
                }

                // Position the pin where the selection is on screen, accounting
                // for the virtual-desktop offset of the captured area (the
                // selection is in screenshot coordinates, which can start at a
                // negative virtual-desktop position on multi-monitor setups).
                if let Some(ref sel) = overlay.selection {
                    let screen_x = sel.x as i32 + overlay.screenshot.x_offset;
                    let screen_y = sel.y as i32 + overlay.screenshot.y_offset;
                    if let Some(pin) =
                        PinnedWindow::create(event_loop, &pixels, w, h, screen_x, screen_y)
                    {
                        self.pinned_windows.push(pin);
                    }
                }
            }
        }
        self.close_overlay();
    }

    fn process_tray_events(&mut self, event_loop: &ActiveEventLoop) {
        let pending = std::mem::take(&mut self.pending_tray);
        for event in pending.into_iter().chain(std::iter::from_fn(|| {
            TrayIconEvent::receiver().try_recv().ok()
        })) {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                tracing::info!("Tray icon clicked — triggering capture");
                self.show_home(event_loop);
            }
        }

        let pending = std::mem::take(&mut self.pending_menu);
        for event in pending
            .into_iter()
            .chain(std::iter::from_fn(|| MenuEvent::receiver().try_recv().ok()))
        {
            if let Some(ref tray) = self.tray {
                if event.id == tray.capture_id {
                    tracing::info!("Capture menu item clicked");
                    self.trigger_capture(event_loop);
                } else if event.id == tray.window_capture_id {
                    tracing::info!("Window capture menu item clicked");
                    self.capture_mode = CaptureMode::Window;
                    self.window_capture_mode = true;
                    self.window_rects = window_detect::enumerate_window_rects();
                    tracing::info!("Enumerated {} windows", self.window_rects.len());
                    self.trigger_capture(event_loop);
                } else if event.id == tray.delay_3_id {
                    tracing::info!("Capturing in 3 seconds...");
                    // Don't set capture_at — countdown reaching 0 triggers the capture
                    self.countdown = Countdown::start(event_loop, 3);
                } else if event.id == tray.delay_5_id {
                    tracing::info!("Capturing in 5 seconds...");
                    self.countdown = Countdown::start(event_loop, 5);
                } else if event.id == tray.delay_10_id {
                    tracing::info!("Capturing in 10 seconds...");
                    self.countdown = Countdown::start(event_loop, 10);
                } else if event.id == tray.autostart_id {
                    let new_state = !snipping_tool_ul::autostart::is_enabled();
                    if let Err(e) = snipping_tool_ul::autostart::set_enabled(new_state) {
                        tracing::error!("Auto-start toggle failed: {}", e);
                    } else {
                        tracing::info!(
                            "Auto-start {}",
                            if new_state { "enabled" } else { "disabled" }
                        );
                        tray.autostart_check.set_checked(new_state);
                    }
                } else if event.id == tray.history_id {
                    tracing::info!("History menu item clicked");
                    self.open_history(event_loop);
                } else if event.id == tray.quit_id {
                    tracing::info!("Quit requested");
                    event_loop.exit();
                } else if event.id == tray.settings_id {
                    tracing::info!("Settings menu item clicked");
                    self.open_settings(event_loop);
                } else if event.id == tray.about_id {
                    let _ = Notification::new().summary("Snipping Tool UL")
                        .body(&format!("Version {}. Ultralight performance, full-featured snips. MIT; third-party notices included.",env!("CARGO_PKG_VERSION")))
                        .timeout(5000).show();
                }
            }
        }
    }

    fn render_preview(&mut self) {
        if !self.needs_redraw {
            return;
        }
        let Some(window) = self.overlay_window.as_ref() else {
            return;
        };
        let size = window.inner_size();
        if size.width == 0 || size.height == 0 {
            self.needs_redraw = false;
            return;
        }
        let dpi = window.scale_factor();
        if self.preview_frame.as_ref().map_or(true, |p| {
            (p.width(), p.height()) != (size.width, size.height)
        }) {
            self.preview_frame = tiny_skia::Pixmap::new(size.width, size.height);
        }
        let (Some(frame), Some(surface), AppState::Capturing(overlay)) = (
            &mut self.preview_frame,
            &mut self.native_surface,
            &mut self.state,
        ) else {
            return;
        };
        self.editor_ui.draw_base(frame, dpi as f32);
        self.preview_renderer
            .draw(overlay, frame, &self.editor_view, dpi);
        let zoom = self
            .editor_view
            .framed_layout(
                overlay.screenshot.width,
                overlay.screenshot.height,
                size.width,
                size.height,
                dpi,
            )
            .scale;
        self.editor_ui
            .draw_controls(frame, dpi as f32, overlay, zoom);
        if let Err(e) = surface.present(frame) {
            tracing::error!("Editor present failed: {e}");
            return;
        }
        if !window.is_visible().unwrap_or(true) {
            window.set_visible(true);
        }
        self.needs_redraw = false;
    }

    fn render(&mut self) {
        if self.is_preview {
            self.render_preview();
            return;
        }
        if !self.needs_redraw {
            return;
        }

        let overlay = match &mut self.state {
            AppState::Capturing(o) => o,
            AppState::Idle => {
                self.needs_redraw = false;
                return;
            }
        };

        let pixmap = match self.pixmap.as_mut() {
            Some(p) => p,
            None => return,
        };

        let surface = match self.surface.as_mut() {
            Some(s) => s,
            None => return,
        };

        let window = match self.overlay_window.as_ref() {
            Some(w) => w,
            None => return,
        };

        render_overlay(overlay, pixmap);

        let size = window.inner_size();
        let w = size.width;
        let h = size.height;

        if w == 0 || h == 0 {
            return;
        }

        if let (Some(nz_w), Some(nz_h)) = (NonZeroU32::new(w), NonZeroU32::new(h)) {
            if let Err(e) = surface.resize(nz_w, nz_h) {
                tracing::error!("Surface resize failed: {e}");
                return;
            }
        }

        let mut buffer = match surface.buffer_mut() {
            Ok(b) => b,
            Err(e) => {
                tracing::error!("Failed to get surface buffer: {e}");
                return;
            }
        };

        // Fast pixmap → softbuffer copy using raw bytes.
        // Screenshot pixels are fully opaque (alpha=255), and tiny-skia composites
        // onto an opaque background, so all pixels remain opaque after rendering.
        // For opaque pixels, premultiplied == straight — skip demultiply entirely.
        // Pixel format: tiny-skia RGBA bytes → softbuffer 0x00RRGGBB u32
        if self.is_preview {
            self.editor_view.present(
                pixmap,
                &mut buffer,
                w,
                h,
                overlay.screenshot.width,
                overlay.screenshot.height,
                snipping_tool_ul::preview::controls_width(overlay.visible_buttons.len()),
                window.scale_factor(),
            );
        } else {
            snipping_tool_ul::preview::present(pixmap, &mut buffer, w, h);
        }

        if let Err(e) = buffer.present() {
            tracing::error!("Buffer present failed: {e}");
        }

        // Show window after first frame is rendered (avoids white flash)
        if let Some(ref window) = self.overlay_window {
            if !window.is_visible().unwrap_or(true) {
                window.set_visible(true);
            }
        }

        self.needs_redraw = false;
    }
}

/// Determine the appropriate cursor icon based on the current overlay state and mouse position.
fn determine_cursor(
    overlay: &OverlayState,
    pos: Point,
    preview: bool,
    hit_slop: f32,
) -> CursorIcon {
    // Text input active — always show text cursor
    if overlay.text_input_active {
        return CursorIcon::Text;
    }

    if preview && overlay.selection_hit(pos, true).is_none() {
        return CursorIcon::Default;
    }
    // Check toolbar hover (only on the legacy capture overlay)
    if let Some(ref sel) = overlay.selection.filter(|_| !preview) {
        let visible_count = overlay.visible_buttons.len();
        let toolbar = overlay.toolbar(sel);
        if toolbar.hit_test_dynamic(pos, visible_count).is_some() {
            return CursorIcon::Pointer;
        }
    }

    // Currently dragging a resize handle on an annotation
    if let Some(handle) = overlay.resize_handle {
        return match handle {
            ResizeHandle::TopLeft | ResizeHandle::BottomRight => CursorIcon::NwseResize,
            ResizeHandle::TopRight | ResizeHandle::BottomLeft => CursorIcon::NeswResize,
        };
    }

    // Hovering over a resize handle on a selected annotation
    if overlay.active_tool == ToolKind::Select {
        if let Some(idx) = overlay.selected_index {
            if let Some(ann) = overlay.annotations.get(idx) {
                if let Some((bx, by, bw, bh)) = annotation_bounding_box(ann) {
                    let handles = [
                        (Point::new(bx, by), ResizeHandle::TopLeft),
                        (Point::new(bx + bw, by), ResizeHandle::TopRight),
                        (Point::new(bx, by + bh), ResizeHandle::BottomLeft),
                        (Point::new(bx + bw, by + bh), ResizeHandle::BottomRight),
                    ];
                    for (hp, handle) in &handles {
                        if (pos.x - hp.x).abs() < hit_slop && (pos.y - hp.y).abs() < hit_slop {
                            return match handle {
                                ResizeHandle::TopLeft | ResizeHandle::BottomRight => {
                                    CursorIcon::NwseResize
                                }
                                ResizeHandle::TopRight | ResizeHandle::BottomLeft => {
                                    CursorIcon::NeswResize
                                }
                            };
                        }
                    }
                }
            }
        }
    }

    // Currently dragging to create selection
    if overlay.is_selecting {
        return CursorIcon::Crosshair;
    }

    // Currently dragging a resize/move zone
    if let Some(zone) = overlay.drag_zone {
        return hitzone_to_cursor(zone, overlay);
    }

    // Selection exists — hit-test it
    if overlay.selection.is_some() {
        if let Some(zone) = overlay.selection_hit(pos, preview) {
            return hitzone_to_cursor(zone, overlay);
        }
    }

    // No selection yet (idle) — crosshair
    CursorIcon::Crosshair
}

/// Map a HitZone to the appropriate cursor icon.
fn hitzone_to_cursor(zone: HitZone, overlay: &OverlayState) -> CursorIcon {
    match zone {
        HitZone::TopLeft | HitZone::BottomRight => CursorIcon::NwseResize,
        HitZone::TopRight | HitZone::BottomLeft => CursorIcon::NeswResize,
        HitZone::Top | HitZone::Bottom => CursorIcon::NsResize,
        HitZone::Left | HitZone::Right => CursorIcon::EwResize,
        HitZone::Inside => {
            // Inside selection — cursor depends on active tool
            match overlay.active_tool {
                ToolKind::Select => CursorIcon::Default,
                ToolKind::Text => CursorIcon::Text,
                ToolKind::StepMarker => CursorIcon::Cell,
                ToolKind::Eyedropper => CursorIcon::Crosshair,
                ToolKind::Arrow | ToolKind::Line | ToolKind::Pencil | ToolKind::Measurement => {
                    CursorIcon::Crosshair
                }
                ToolKind::Rectangle
                | ToolKind::RoundedRect
                | ToolKind::Circle
                | ToolKind::Highlight
                | ToolKind::Pixelate
                | ToolKind::Spotlight => CursorIcon::Crosshair,
            }
        }
    }
}

/// Encode RGBA pixels to PNG bytes.
fn encode_png(pixels: Vec<u8>, w: u32, h: u32) -> Result<Vec<u8>, String> {
    let img = image::RgbaImage::from_raw(w, h, pixels).ok_or("invalid image data")?;
    let mut png_bytes = Vec::new();
    img.write_to(
        &mut std::io::Cursor::new(&mut png_bytes),
        image::ImageFormat::Png,
    )
    .map_err(|e| e.to_string())?;
    Ok(png_bytes)
}

/// Build a hotkey binding string (e.g. "Ctrl+Shift+S") from the current
/// modifier state and a pressed key. Returns None for keys that can't be a
/// global hotkey or when no Ctrl/Alt/Super modifier is held.
fn hotkey_binding_from_key(mods: ModifiersState, key: &Key) -> Option<String> {
    let key_part: String = match key {
        Key::Character(ch) => {
            let s = ch.as_str();
            let c = s.chars().next()?;
            if s.chars().count() == 1 && (c.is_ascii_alphabetic() || c.is_ascii_digit()) {
                c.to_ascii_uppercase().to_string()
            } else {
                return None;
            }
        }
        Key::Named(named) => match named {
            NamedKey::F1 => "F1".into(),
            NamedKey::F2 => "F2".into(),
            NamedKey::F3 => "F3".into(),
            NamedKey::F4 => "F4".into(),
            NamedKey::F5 => "F5".into(),
            NamedKey::F6 => "F6".into(),
            NamedKey::F7 => "F7".into(),
            NamedKey::F8 => "F8".into(),
            NamedKey::F9 => "F9".into(),
            NamedKey::F10 => "F10".into(),
            NamedKey::F11 => "F11".into(),
            NamedKey::F12 => "F12".into(),
            NamedKey::PrintScreen => "PrintScreen".into(),
            _ => return None,
        },
        _ => return None,
    };

    // Require a strong modifier so the hotkey can't swallow normal typing.
    if !(mods.control_key() || mods.alt_key() || mods.super_key()) {
        return None;
    }

    let mut parts: Vec<String> = Vec::new();
    if mods.control_key() {
        parts.push("Ctrl".into());
    }
    if mods.shift_key() {
        parts.push("Shift".into());
    }
    if mods.alt_key() {
        parts.push("Alt".into());
    }
    if mods.super_key() {
        parts.push("Super".into());
    }
    parts.push(key_part);
    let binding = parts.join("+");

    // Final validation through the real parser
    snipping_tool_ul::hotkey::parse_binding(&binding).ok()?;
    Some(binding)
}

impl ApplicationHandler<UserEvent> for App {
    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::UploadFinished {
                revision,
                key,
                url,
                message,
            } => {
                let upload_done = self.upload_session.finish(key, url);
                self.user_event(
                    event_loop,
                    UserEvent::Status {
                        revision,
                        message,
                        upload_done,
                    },
                );
            }
            UserEvent::SavePathReady { revision, path } => {
                self.save_dialog_pending = false;
                if let Some(path) = path {
                    if revision == self.image_revision {
                        self.save_chosen_path(path);
                    } else {
                        self.user_event(
                            event_loop,
                            UserEvent::Status {
                                revision: self.image_revision,
                                message: "Capture changed. Click Save again for the current image."
                                    .into(),
                                upload_done: false,
                            },
                        );
                    }
                }
            }
            UserEvent::IntegrationReady(result) => {
                self.integration_pending = false;
                self.integration_status=match result {Ok(_)=>"Installed. Search Start for Snipping Tool. Restart UL to use the installed copy.".into(),Err(e)=>format!("Could not finish: {e}")};
                if let Some(sw) = &mut self.settings_window {
                    sw.integration_pending = false;
                    sw.integration_status = self.integration_status.clone();
                    sw.window.request_redraw();
                }
            }
            UserEvent::Menu(event) => self.pending_menu.push(event),
            UserEvent::Tray(event) => self.pending_tray.push(event),
            UserEvent::Hotkey(event) => {
                if event.state == global_hotkey::HotKeyState::Pressed
                    && Some(event.id) == self.hotkey_id
                {
                    self.trigger_capture(event_loop);
                }
            }
            UserEvent::ShowHome => self.show_home(event_loop),
            UserEvent::Status {
                revision,
                message,
                upload_done,
            } => {
                if upload_done {
                    self.upload_pending = false;
                }
                if revision == self.image_revision {
                    self.editor_ui.header.status = message.clone();
                    if let AppState::Capturing(ref mut o) = self.state {
                        o.show_toast(message.clone(), 8000);
                        self.needs_redraw = true;
                    }
                    if let Some(h) = &mut self.launcher {
                        h.model.status = message;
                        h.window.request_redraw();
                    }
                }
            }
            UserEvent::SnipReady(result) => {
                self.picker_active = false;
                let restore_home = std::mem::take(&mut self.restore_home_on_cancel);
                match result {
                    Ok(Some(snip)) => {
                        let copied = snip.clipboard_error.is_none();
                        if let Some(error) = snip.clipboard_error {
                            tracing::error!("Clipboard copy failed: {error}");
                            let _ = Notification::new()
                                .summary("Snipping Tool UL")
                                .body(&format!(
                                    "Copy failed: {error}. Use Copy in the preview to retry."
                                ))
                                .show();
                        }
                        if self.config.workflow.auto_save {
                            let pixels = snip.screen.pixels.clone();
                            let (w, h) = (snip.screen.width, snip.screen.height);
                            let dir = self.config.auto_save_directory();
                            let proxy = self.proxy.clone();
                            let revision = self.image_revision;
                            std::thread::spawn(move || {
                                let saved = dir
                                    .and_then(|dir| export::save_to_file(pixels, w, h, Some(&dir)));
                                if let Err(e) = saved {
                                    let _ = proxy.send_event(UserEvent::Status {
                                        revision,
                                        message: format!("Auto-save failed: {e}"),
                                        upload_done: false,
                                    });
                                }
                            });
                        }
                        // A failed copy always keeps a recoverable preview.
                        if !self.config.workflow.open_preview && copied {
                            if self.config.general.history_enabled {
                                let screen = snip.screen;
                                std::thread::spawn(move || {
                                    let _ = snipping_tool_ul::history::save_to_history(
                                        &screen.pixels,
                                        screen.width,
                                        screen.height,
                                    );
                                });
                            }
                            if self.cli_only {
                                event_loop.exit();
                            }
                            return;
                        }
                        if let Err(error) = self.open_preview(event_loop, snip.screen, copied) {
                            tracing::error!("Preview failed: {error}");
                            let _ = Notification::new()
                                .summary("Snipping Tool UL")
                                .body(&format!("Preview failed: {error}"))
                                .show();
                            if self.cli_only {
                                event_loop.exit();
                            }
                        }
                    }
                    Ok(None) => {
                        if self.cli_only {
                            event_loop.exit();
                        } else if restore_home {
                            self.show_home(event_loop);
                        }
                    }
                    Err(error) => {
                        tracing::error!("Capture failed: {error}");
                        let _ = Notification::new()
                            .summary("Snipping Tool UL")
                            .body(&format!("Capture failed: {error}"))
                            .show();
                        if self.cli_only {
                            event_loop.exit();
                        } else if restore_home {
                            self.show_home(event_loop);
                        }
                    }
                }
            }
        }
    }
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        tracing::info!("Application resumed");
        event_loop.set_control_flow(ControlFlow::Wait);

        if !self.cli_only {
            if self.tray.is_none() {
                match tray::create_tray() {
                    Ok(t) => {
                        tracing::info!("Tray icon created");
                        self.tray = Some(t);
                    }
                    Err(e) => {
                        tracing::error!("Failed to create tray icon: {e}");
                    }
                }
            }

            if self._hotkey_manager.is_none() {
                self.register_hotkey_with_fallback();
            }
        }

        if self.show_home_on_start {
            self.show_home_on_start = false;
            self.show_home(event_loop);
        }

        if self.immediate_capture {
            self.immediate_capture = false;
            self.trigger_capture(event_loop);
        }
    }

    fn window_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        if self
            .launcher
            .as_ref()
            .is_some_and(|h| h.window.id() == _window_id)
        {
            let mut action = None;
            match event {
                WindowEvent::CloseRequested => {
                    if self.config.general.quit_on_close {
                        _event_loop.exit();
                    }
                    self.launcher = None;
                    return;
                }
                WindowEvent::RedrawRequested
                | WindowEvent::Resized(_)
                | WindowEvent::ScaleFactorChanged { .. } => {
                    if let Some(h) = &mut self.launcher {
                        let _ = h.render();
                    }
                }
                WindowEvent::CursorMoved { position, .. } => {
                    if let Some(h) = &mut self.launcher {
                        h.pointer = h.point(position.x, position.y);
                        if h.model.hover(h.pointer) {
                            h.window.request_redraw();
                        }
                    }
                }
                WindowEvent::CursorLeft { .. } => {
                    if let Some(h) = &mut self.launcher {
                        if h.model.hover(Point::new(-1., -1.)) {
                            h.window.request_redraw();
                        }
                    }
                }
                WindowEvent::ModifiersChanged(m) => self.modifiers = m.state(),
                WindowEvent::MouseInput {
                    state: ElementState::Released,
                    button: WinitMouseButton::Left,
                    ..
                } => {
                    if let Some(h) = &self.launcher {
                        action = h.model.hit(h.pointer);
                    }
                }
                WindowEvent::KeyboardInput { event, .. }
                    if event.state == ElementState::Pressed =>
                {
                    if let Some(h) = &mut self.launcher {
                        if h.model.connecting {
                            match &event.logical_key {
                                Key::Named(NamedKey::Escape) => {
                                    action = Some(HomeAction::CancelConnection)
                                }
                                Key::Named(NamedKey::Enter) => {
                                    action = Some(HomeAction::SaveConnection)
                                }
                                Key::Named(NamedKey::Backspace) => {
                                    h.model.draft.pop();
                                }
                                Key::Character(c)
                                    if self.modifiers.control_key()
                                        && c.eq_ignore_ascii_case("v") =>
                                {
                                    if let Ok(mut clipboard) = arboard::Clipboard::new() {
                                        if let Ok(text) = clipboard.get_text() {
                                            h.model.append(&text);
                                        }
                                    }
                                }
                                Key::Character(_) if self.modifiers.control_key() => {}
                                Key::Character(c) => h.model.append(c),
                                _ => {}
                            }
                        } else {
                            match &event.logical_key {
                                Key::Named(NamedKey::Enter) => {
                                    action = h.model.enter(snipping_tool_ul::launcher::WIDTH as f32)
                                }
                                Key::Named(NamedKey::ArrowDown) | Key::Named(NamedKey::Tab) => {
                                    h.model.navigate(
                                        if self.modifiers.shift_key() { -1 } else { 1 },
                                        snipping_tool_ul::launcher::WIDTH as f32,
                                    );
                                }
                                Key::Named(NamedKey::ArrowUp) => {
                                    h.model
                                        .navigate(-1, snipping_tool_ul::launcher::WIDTH as f32);
                                }
                                Key::Named(NamedKey::Escape)
                                    if h.model.menu != snipping_tool_ul::launcher::Menu::None =>
                                {
                                    h.model.menu = snipping_tool_ul::launcher::Menu::None;
                                }
                                Key::Named(NamedKey::Escape) => {
                                    self.launcher = None;
                                    return;
                                }
                                _ => {}
                            }
                        }
                        h.window.request_redraw();
                    }
                }
                _ => {}
            }
            if let Some(action) = action {
                let handled = self
                    .launcher
                    .as_mut()
                    .is_some_and(|h| h.model.activate(action));
                if handled {
                    if let Some(h) = &self.launcher {
                        h.window.request_redraw();
                    }
                } else {
                    self.home_action(_event_loop, action);
                }
            }
            return;
        }
        // Check if event is for a pinned window
        if let Some(pin_idx) = self
            .pinned_windows
            .iter()
            .position(|p| p.window_id() == _window_id)
        {
            if self.pinned_windows[pin_idx].handle_event(&event) {
                // PinnedWindow::drop removes the backing temp file.
                self.pinned_windows.remove(pin_idx);
                tracing::info!("Pinned window closed");
            }
            return;
        }

        // Check if event is for the history window
        if let Some(ref hw) = self.history_window {
            if hw.window.id() == _window_id {
                match event {
                    WindowEvent::CloseRequested => {
                        self.close_history();
                        return;
                    }
                    WindowEvent::RedrawRequested => {
                        if let Some(ref mut hw) = self.history_window {
                            hw.render();
                        }
                        return;
                    }
                    WindowEvent::CursorMoved { position, .. } => {
                        if let Some(ref mut hw) = self.history_window {
                            if hw.on_cursor_moved(position.x as f32, position.y as f32) {
                                hw.needs_redraw = true;
                                hw.window.request_redraw();
                            }
                        }
                        return;
                    }
                    WindowEvent::MouseInput {
                        state: ElementState::Pressed,
                        button: WinitMouseButton::Left,
                        ..
                    } => {
                        if let Some(ref mut hw) = self.history_window {
                            if hw.on_click() {
                                hw.render();
                            }
                        }
                        return;
                    }
                    WindowEvent::MouseWheel { delta, .. } => {
                        let scroll = match delta {
                            MouseScrollDelta::LineDelta(_, y) => y,
                            MouseScrollDelta::PixelDelta(pos) => pos.y as f32 / 20.0,
                        };
                        if let Some(ref mut hw) = self.history_window {
                            if hw.on_scroll(scroll) {
                                hw.window.request_redraw();
                            }
                        }
                        return;
                    }
                    WindowEvent::KeyboardInput { ref event, .. }
                        if event.state == ElementState::Pressed =>
                    {
                        if let Key::Named(NamedKey::Escape) = &event.logical_key {
                            self.close_history();
                            return;
                        }
                    }
                    _ => {}
                }
                return;
            }
        }

        // Check if event is for the settings window
        if let Some(ref sw) = self.settings_window {
            if sw.window.id() == _window_id {
                match event {
                    WindowEvent::CloseRequested => {
                        self.close_settings(false);
                        return;
                    }
                    WindowEvent::RedrawRequested
                    | WindowEvent::Resized(_)
                    | WindowEvent::ScaleFactorChanged { .. } => {
                        if let Some(ref mut sw) = self.settings_window {
                            sw.render();
                        }
                        return;
                    }
                    WindowEvent::CursorMoved { position, .. } => {
                        if let Some(ref mut sw) = self.settings_window {
                            if sw.on_cursor_moved(position.x as f32, position.y as f32) {
                                sw.needs_redraw = true;
                                sw.window.request_redraw();
                            }
                        }
                        return;
                    }
                    WindowEvent::MouseInput {
                        state: ElementState::Pressed,
                        button: WinitMouseButton::Left,
                        ..
                    } => {
                        let (cx, cy) = if let Some(ref sw) = self.settings_window {
                            (sw.cursor_pos.0, sw.cursor_pos.1)
                        } else {
                            return;
                        };
                        let action = if let Some(ref sw) = self.settings_window {
                            sw.on_click(cx, cy)
                        } else {
                            None
                        };
                        if let Some(action) = action {
                            if matches!(
                                action,
                                snipping_tool_ul::settings_ui::Action::TakeOverWindows
                            ) {
                                if self.integration_pending {
                                    return;
                                }
                                let answer=rfd::MessageDialog::new().set_title("Use Snipping Tool UL in Start search?")
                                    .set_description("Install UL for your account and remove Microsoft's Snipping Tool for this account? UL will be named Snipping Tool in Start. Microsoft can be restored from the Store. Win+Shift+S is not remapped; use your configured capture hotkey.")
                                    .set_buttons(rfd::MessageButtons::YesNo).show();
                                if answer == rfd::MessageDialogResult::Yes {
                                    self.integration_pending = true;
                                    self.integration_status =
                                        "Installing and updating Start...".into();
                                    if let Some(sw) = &mut self.settings_window {
                                        sw.integration_pending = true;
                                        sw.integration_status =
                                            "Installing and updating Start...".into();
                                        sw.window.request_redraw();
                                    }
                                    let proxy = self.proxy.clone();
                                    std::thread::spawn(move || {
                                        let result = std::env::current_exe()
                                            .map_err(|e| e.to_string())
                                            .and_then(|exe| {
                                                snipping_tool_ul::windows_integration::install(
                                                    &exe, true, true, false,
                                                )
                                            });
                                        let _ =
                                            proxy.send_event(UserEvent::IntegrationReady(result));
                                    });
                                }
                                return;
                            }
                            if matches!(
                                action,
                                snipping_tool_ul::settings_ui::Action::RestoreWindows
                            ) {
                                if let Err(e) =
                                    snipping_tool_ul::windows_integration::restore_store()
                                {
                                    if let Some(sw) = &mut self.settings_window {
                                        sw.integration_status = e;
                                        sw.window.request_redraw();
                                    }
                                }
                                return;
                            }
                            let should_close = if let Some(ref mut sw) = self.settings_window {
                                sw.config.workflow = self.config.workflow.clone();
                                sw.config.general.imgur_client_id =
                                    self.config.general.imgur_client_id.clone();
                                sw.handle_action(action)
                            } else {
                                false
                            };
                            if should_close {
                                self.close_settings(true);
                            } else if let Some(ref sw) = self.settings_window {
                                sw.window.request_redraw();
                            }
                        }
                        return;
                    }
                    WindowEvent::ModifiersChanged(mods) => {
                        self.settings_modifiers = mods.state();
                        return;
                    }
                    WindowEvent::KeyboardInput { ref event, .. }
                        if event.state == ElementState::Pressed =>
                    {
                        // If rebinding the global hotkey, capture the combo
                        if let Some(ref mut sw) = self.settings_window {
                            if sw.editing_hotkey {
                                if let Key::Named(NamedKey::Escape) = &event.logical_key {
                                    sw.editing_hotkey = false;
                                    sw.needs_redraw = true;
                                    sw.window.request_redraw();
                                    return;
                                }
                                if let Some(binding) = hotkey_binding_from_key(
                                    self.settings_modifiers,
                                    &event.logical_key,
                                ) {
                                    sw.config.hotkey.capture = binding;
                                    sw.editing_hotkey = false;
                                    sw.needs_redraw = true;
                                    sw.window.request_redraw();
                                }
                                // Bare keys / unsupported keys keep edit mode active
                                return;
                            }
                        }

                        // If editing a shortcut, capture the key
                        if let Some(ref mut sw) = self.settings_window {
                            if sw.editing_shortcut.is_some() {
                                let key_str = match &event.logical_key {
                                    Key::Character(ch) => Some(ch.as_str().to_lowercase()),
                                    Key::Named(NamedKey::Escape) => {
                                        // Cancel editing
                                        sw.editing_shortcut = None;
                                        sw.needs_redraw = true;
                                        sw.window.request_redraw();
                                        None
                                    }
                                    _ => None,
                                };
                                if let Some(key) = key_str {
                                    sw.on_key_press(&key);
                                    sw.window.request_redraw();
                                }
                                return;
                            }
                        }

                        if let Key::Named(NamedKey::Escape) = &event.logical_key {
                            self.close_settings(false);
                            return;
                        }
                    }
                    _ => {}
                }
                return;
            }
        }

        // Check if event is for the countdown window
        if let Some(ref mut cd) = self.countdown {
            if cd.window_id() == _window_id {
                match event {
                    WindowEvent::CloseRequested => {
                        self.countdown = None;
                        self.capture_at = None; // cancel the delayed capture
                        if std::mem::take(&mut self.restore_home_on_cancel) {
                            self.show_home(_event_loop);
                        }
                        return;
                    }
                    WindowEvent::KeyboardInput { ref event, .. }
                        if event.state == ElementState::Pressed
                            && event.logical_key == Key::Named(NamedKey::Escape) =>
                    {
                        self.countdown = None;
                        self.capture_at = None;
                        // New hid the home window; a cancelled timer brings it back.
                        if std::mem::take(&mut self.restore_home_on_cancel) {
                            self.show_home(_event_loop);
                        }
                        return;
                    }
                    WindowEvent::RedrawRequested => {
                        cd.render();
                        return;
                    }
                    _ => {}
                }
                return;
            }
        }

        // Check if event is for the overlay window
        if let Some(ref overlay_win) = self.overlay_window {
            if overlay_win.id() != _window_id {
                return; // Unknown window, ignore
            }
        }

        if self.is_preview
            && self
                .overlay_window
                .as_ref()
                .is_some_and(|w| w.id() == _window_id)
        {
            let window = self.overlay_window.as_ref().unwrap();
            let dpi = window.scale_factor();
            let size = window.inner_size();
            let (w, h) = (
                size.width as f32 / dpi as f32,
                size.height as f32 / dpi as f32,
            );
            let mut requested = None;
            let mut consumed = false;
            if let AppState::Capturing(ref mut o) = self.state {
                let hit_slop = self.editor_view.annotation_hit_slop(
                    o.screenshot.width,
                    o.screenshot.height,
                    size.width,
                    size.height,
                    dpi,
                );
                match &event {
                    WindowEvent::MouseInput {
                        state: ElementState::Pressed,
                        button: WinitMouseButton::Middle,
                        ..
                    } => {
                        if !self.editor_pointer_down
                            && !o.text_input_active
                            && !self.editor_ui.size_dragging
                            && self.editor_ui.size_draft.is_none()
                            && self.editor_ui.header.menu == snipping_tool_ul::launcher::Menu::None
                            && self.editor_view.begin_pan(
                                self.editor_pointer.0,
                                self.editor_pointer.1,
                                size.width,
                                size.height,
                                dpi,
                            )
                        {
                            self.editor_ui.hover(Point::new(-1., -1.), w, h, o);
                            window.set_cursor(CursorIcon::Grabbing);
                            self.needs_redraw = true;
                        }
                        consumed = true;
                    }
                    WindowEvent::MouseInput {
                        state: ElementState::Released,
                        button: WinitMouseButton::Middle,
                        ..
                    } => {
                        self.editor_view.end_pan();
                        window.set_cursor(determine_cursor(o, o.last_mouse_pos, true, hit_slop));
                        consumed = true;
                    }
                    WindowEvent::CursorMoved { position, .. } if self.editor_view.is_panning() => {
                        self.editor_pointer = (position.x, position.y);
                        self.needs_redraw |= self.editor_view.drag_pan(position.x, position.y);
                        o.last_mouse_pos = self.editor_view.framed_point(
                            position.x,
                            position.y,
                            o.screenshot.width,
                            o.screenshot.height,
                            size.width,
                            size.height,
                            dpi,
                        );
                        window.set_cursor(CursorIcon::Grabbing);
                        consumed = true;
                    }
                    WindowEvent::Focused(false) | WindowEvent::CursorLeft { .. } => {
                        if self.editor_view.end_pan() {
                            window.set_cursor(CursorIcon::Default);
                        }
                    }
                    WindowEvent::MouseInput { .. } | WindowEvent::MouseWheel { .. }
                        if self.editor_view.is_panning() =>
                    {
                        consumed = true
                    }
                    _ => {}
                }
            }
            if consumed {
                if self.needs_redraw {
                    window.request_redraw();
                }
                return;
            }
            if matches!(
                &event,
                WindowEvent::MouseInput {
                    state: ElementState::Released,
                    button: WinitMouseButton::Left,
                    ..
                }
            ) {
                self.editor_pointer_down = false;
            }
            if let AppState::Capturing(ref mut o) = self.state {
                match &event {
                    WindowEvent::KeyboardInput { event, .. }
                        if event.state == ElementState::Pressed
                            && self.editor_ui.size_draft.is_some() =>
                    {
                        let draft = self.editor_ui.size_draft.as_mut().unwrap();
                        match &event.logical_key {
                            Key::Named(NamedKey::Escape) => self.editor_ui.size_draft = None,
                            Key::Named(NamedKey::Enter) => {
                                if let Ok(v) = draft.parse::<f32>() {
                                    if v.is_finite() && v > 0. {
                                        requested = Some(HomeAction::SetSize(
                                            (v.clamp(0., 128.) * 10.).round() as u16,
                                        ));
                                        self.editor_ui.size_draft = None;
                                    }
                                }
                            }
                            Key::Named(NamedKey::Backspace) => {
                                if self.editor_ui.size_select_all {
                                    draft.clear();
                                } else {
                                    draft.pop();
                                }
                                self.editor_ui.size_select_all = false;
                            }
                            Key::Character(c)
                                if self.modifiers.control_key() && c.eq_ignore_ascii_case("a") =>
                            {
                                self.editor_ui.size_select_all = true;
                            }
                            Key::Character(c) if !self.modifiers.control_key() => {
                                if self.editor_ui.size_select_all {
                                    draft.clear();
                                    self.editor_ui.size_select_all = false;
                                }
                                for ch in c.chars().filter(|c| c.is_ascii_digit() || *c == '.') {
                                    if draft.len() < 6 {
                                        draft.push(ch);
                                    }
                                }
                            }
                            _ => {}
                        }
                        self.needs_redraw = true;
                        consumed = true;
                    }
                    WindowEvent::MouseInput {
                        state: ElementState::Released,
                        button: WinitMouseButton::Left,
                        ..
                    } if self.editor_ui.size_dragging => {
                        self.editor_ui.end_size_drag(o);
                        consumed = true;
                    }
                    WindowEvent::CursorLeft { .. } => {
                        if self.editor_ui.hover(Point::new(-1., -1.), w, h, o) {
                            self.needs_redraw = true;
                        }
                    }
                    WindowEvent::CursorMoved { position, .. } => {
                        self.editor_pointer = (position.x, position.y);
                        let p = Point::new((position.x / dpi) as f32, (position.y / dpi) as f32);
                        if self.editor_ui.size_dragging {
                            requested =
                                snipping_tool_ul::editor_ui::EditorUi::slider_value(p, w, h, o);
                            consumed = true;
                        }
                        if self.editor_ui.hover(p, w, h, o) {
                            self.needs_redraw = true;
                        }
                        if (self.editor_ui.header.menu != snipping_tool_ul::launcher::Menu::None
                            || !snipping_tool_ul::editor_ui::EditorUi::canvas_rect(w, h)
                                .contains(p))
                            && o.drag_start.is_none()
                            && o.select_drag_start.is_none()
                            && !self.editor_pointer_down
                        {
                            window.set_cursor(if self.editor_ui.header.hovered.is_some() {
                                CursorIcon::Pointer
                            } else {
                                CursorIcon::Default
                            });
                            consumed = true;
                        } else if !self.editor_pointer_down
                            && o.select_drag_start.is_none()
                            && o.drag_start.is_none()
                            && o.active_tool != ToolKind::Eyedropper
                        {
                            o.last_mouse_pos = self.editor_view.framed_point(
                                position.x,
                                position.y,
                                o.screenshot.width,
                                o.screenshot.height,
                                size.width,
                                size.height,
                                dpi,
                            );
                            window.set_cursor(determine_cursor(
                                o,
                                o.last_mouse_pos,
                                true,
                                self.editor_view.annotation_hit_slop(
                                    o.screenshot.width,
                                    o.screenshot.height,
                                    size.width,
                                    size.height,
                                    dpi,
                                ),
                            ));
                            consumed = true;
                        }
                    }
                    WindowEvent::MouseInput {
                        state: ElementState::Pressed,
                        button: WinitMouseButton::Left,
                        ..
                    } => {
                        let p = Point::new(
                            (self.editor_pointer.0 / dpi) as f32,
                            (self.editor_pointer.1 / dpi) as f32,
                        );
                        if let Some(a) = self.editor_ui.hit(p, w, h, o) {
                            if matches!(a, HomeAction::SetSize(_)) {
                                self.editor_ui.begin_size_drag(o);
                            }
                            if a != HomeAction::EditSize {
                                self.editor_ui.size_draft = None;
                            }
                            if self.editor_ui.activate(a) {
                                self.needs_redraw = true;
                            } else {
                                requested = Some(a);
                            }
                            consumed = true;
                        } else if !snipping_tool_ul::editor_ui::EditorUi::canvas_rect(w, h)
                            .contains(p)
                        {
                            consumed = true;
                        } else {
                            let point = self.editor_view.framed_point(
                                self.editor_pointer.0,
                                self.editor_pointer.1,
                                o.screenshot.width,
                                o.screenshot.height,
                                size.width,
                                size.height,
                                dpi,
                            );
                            self.editor_pointer_down = o.selection_hit(point, true).is_some();
                        }
                    }
                    WindowEvent::MouseInput {
                        state: ElementState::Pressed,
                        button: WinitMouseButton::Right,
                        ..
                    } => {
                        let point = Point::new(
                            (self.editor_pointer.0 / dpi) as f32,
                            (self.editor_pointer.1 / dpi) as f32,
                        );
                        if self.editor_ui.hit(point, w, h, o) == Some(HomeAction::ToggleColors) {
                            requested = Some(HomeAction::CustomColor);
                        } else {
                            let pos = self.editor_view.framed_point(
                                self.editor_pointer.0,
                                self.editor_pointer.1,
                                o.screenshot.width,
                                o.screenshot.height,
                                size.width,
                                size.height,
                                dpi,
                            );
                            if let Some(i) = o.annotations.iter().rposition(|a| {
                                hit_test_annotation(
                                    a,
                                    &pos,
                                    self.editor_view.annotation_hit_slop(
                                        o.screenshot.width,
                                        o.screenshot.height,
                                        size.width,
                                        size.height,
                                        dpi,
                                    ),
                                )
                            }) {
                                o.selected_index = Some(i);
                                self.editor_ui.annotation_anchor = point;
                                self.editor_ui.header.menu =
                                    snipping_tool_ul::launcher::Menu::Annotation;
                            } else {
                                self.editor_ui.header.menu =
                                    snipping_tool_ul::launcher::Menu::Options;
                            }
                        }
                        self.needs_redraw = true;
                        consumed = true;
                    }
                    WindowEvent::MouseWheel { .. }
                        if self.editor_ui.header.menu != snipping_tool_ul::launcher::Menu::None =>
                    {
                        consumed = true;
                    }
                    WindowEvent::KeyboardInput { event, .. }
                        if event.state == ElementState::Pressed
                            && !o.text_input_active
                            && !self.modifiers.control_key()
                            && matches!(
                                event.logical_key,
                                Key::Named(NamedKey::Tab)
                                    | Key::Named(NamedKey::ArrowDown)
                                    | Key::Named(NamedKey::ArrowUp)
                            )
                            && (event.logical_key == Key::Named(NamedKey::Tab)
                                || self.editor_ui.header.menu
                                    != snipping_tool_ul::launcher::Menu::None) =>
                    {
                        self.editor_ui.navigate(
                            w,
                            h,
                            o,
                            if event.logical_key == Key::Named(NamedKey::ArrowUp)
                                || self.modifiers.shift_key()
                            {
                                -1
                            } else {
                                1
                            },
                        );
                        self.needs_redraw = true;
                        consumed = true;
                    }
                    WindowEvent::KeyboardInput { event, .. }
                        if event.state == ElementState::Pressed
                            && !o.text_input_active
                            && event.logical_key == Key::Named(NamedKey::Enter)
                            && (self.editor_ui.header.keyboard_focus
                                || self.editor_ui.header.menu
                                    != snipping_tool_ul::launcher::Menu::None) =>
                    {
                        if let Some(a) = self.editor_ui.enter(w, h, o) {
                            if self.editor_ui.activate(a) {
                                self.needs_redraw = true;
                            } else {
                                requested = Some(a);
                            }
                        }
                        consumed = true;
                    }
                    WindowEvent::KeyboardInput { event, .. }
                        if event.state == ElementState::Pressed
                            && event.logical_key == Key::Named(NamedKey::Escape)
                            && self.editor_ui.header.menu
                                != snipping_tool_ul::launcher::Menu::None =>
                    {
                        self.editor_ui.header.menu = snipping_tool_ul::launcher::Menu::None;
                        self.needs_redraw = true;
                        consumed = true;
                    }
                    _ => {}
                }
            }
            if let Some(a) = requested {
                self.home_action(_event_loop, a);
            }
            if consumed {
                if self.needs_redraw {
                    if let Some(w) = &self.overlay_window {
                        w.request_redraw();
                    }
                }
                return;
            }
        }
        // Keep selection targets comfortable in viewport pixels, not shrinking image pixels.
        let hit_slop = if self.is_preview {
            match (&self.state, &self.overlay_window) {
                (AppState::Capturing(o), Some(window)) => {
                    let size = window.inner_size();
                    self.editor_view.annotation_hit_slop(
                        o.screenshot.width,
                        o.screenshot.height,
                        size.width,
                        size.height,
                        window.scale_factor(),
                    )
                }
                _ => 9.,
            }
        } else {
            8.
        };
        // Only process events when we're capturing
        let overlay = match &mut self.state {
            AppState::Capturing(o) => o,
            AppState::Idle => return,
        };

        match event {
            WindowEvent::CloseRequested => {
                self.close_overlay();
                if self.cli_only || self.config.general.quit_on_close {
                    _event_loop.exit();
                }
                return;
            }

            WindowEvent::RedrawRequested => {
                self.needs_redraw = true;
                if !self.is_preview {
                    self.render();
                }
                return;
            }

            WindowEvent::Resized(_) => {
                self.needs_redraw = true;
            }

            WindowEvent::ModifiersChanged(mods) => {
                self.modifiers = mods.state();
            }

            WindowEvent::KeyboardInput { event, .. } => {
                if event.state != ElementState::Pressed {
                    return;
                }

                if self.is_preview
                    && self.modifiers.control_key()
                    && matches!(&event.logical_key,Key::Character(c) if c=="0")
                    && !overlay.text_input_active
                {
                    self.editor_view = Default::default();
                    self.needs_redraw = true;
                    if let Some(w) = &self.overlay_window {
                        w.request_redraw();
                    }
                    return;
                }
                // --- Text input guard: MUST be first ---
                if overlay.text_input_active {
                    let ctrl = self.modifiers.control_key();
                    match &event.logical_key {
                        Key::Character(ch) if ctrl && ch.as_str().eq_ignore_ascii_case("v") => {
                            // Ctrl+V: paste clipboard text into the buffer
                            if let Ok(mut clipboard) = arboard::Clipboard::new() {
                                if let Ok(text) = clipboard.get_text() {
                                    // Single-line annotation — fold newlines
                                    let text = text.replace(['\r', '\n'], " ");
                                    overlay.text_input_buffer.push_str(&text);
                                    self.needs_redraw = true;
                                }
                            }
                        }
                        Key::Character(_) if ctrl => {
                            // Swallow other Ctrl chords so they don't insert
                            // literal characters while typing.
                        }
                        Key::Character(ch) => {
                            overlay.text_input_buffer.push_str(ch.as_str());
                            self.needs_redraw = true;
                        }
                        Key::Named(NamedKey::Space) => {
                            overlay.text_input_buffer.push(' ');
                            self.needs_redraw = true;
                        }
                        Key::Named(NamedKey::Backspace) => {
                            overlay.text_input_buffer.pop();
                            self.needs_redraw = true;
                        }
                        Key::Named(NamedKey::Enter) => {
                            let buffer = overlay.text_input_buffer.clone();
                            if let Some((idx, old_ann)) = overlay.text_edit_origin.take() {
                                // Re-edit of an existing annotation: one Modify
                                // step (or a Delete when the text was cleared).
                                let idx = idx.min(overlay.annotations.len());
                                if !buffer.is_empty() {
                                    let ann = Annotation::Text {
                                        position: overlay.text_input_position,
                                        text: buffer,
                                        color: overlay.current_color,
                                        font_size: overlay.text_input_font_size,
                                    };
                                    overlay.annotations.insert(idx, ann);
                                    record_undo(
                                        &mut overlay.undo_stack,
                                        &mut overlay.redo_stack,
                                        UndoAction::Modify(idx, old_ann),
                                    );
                                } else {
                                    record_undo(
                                        &mut overlay.undo_stack,
                                        &mut overlay.redo_stack,
                                        UndoAction::Delete(idx, old_ann),
                                    );
                                }
                            } else if !buffer.is_empty() {
                                let ann = Annotation::Text {
                                    position: overlay.text_input_position,
                                    text: buffer,
                                    color: overlay.current_color,
                                    font_size: overlay.text_input_font_size,
                                };
                                overlay.annotations.push(ann);
                                record_undo(
                                    &mut overlay.undo_stack,
                                    &mut overlay.redo_stack,
                                    UndoAction::Add(overlay.annotations.len() - 1),
                                );
                            }
                            overlay.text_input_buffer.clear();
                            overlay.text_input_active = false;
                            self.needs_redraw = true;
                        }
                        Key::Named(NamedKey::Escape) => {
                            // Cancel text input; restore the original annotation
                            // if this was a re-edit.
                            if let Some((idx, old_ann)) = overlay.text_edit_origin.take() {
                                let idx = idx.min(overlay.annotations.len());
                                overlay.annotations.insert(idx, old_ann);
                            }
                            overlay.text_input_buffer.clear();
                            overlay.text_input_active = false;
                            self.needs_redraw = true;
                        }
                        _ => {
                            // Ignore all other keys while typing
                        }
                    }
                    // Return early — don't fall through to other handlers
                    if self.needs_redraw {
                        if let Some(w) = &self.overlay_window {
                            w.request_redraw();
                        }
                    }
                    return;
                }

                match &event.logical_key {
                    Key::Named(NamedKey::Escape) => {
                        let overlay = match &mut self.state {
                            AppState::Capturing(o) => o,
                            _ => return,
                        };
                        if overlay.selected_index.is_some() {
                            overlay.selected_index = None;
                            overlay.select_drag_start = None;
                            self.needs_redraw = true;
                        } else {
                            self.close_overlay();
                            if self.cli_only {
                                _event_loop.exit();
                            }
                            return;
                        }
                    }
                    Key::Named(NamedKey::Delete) | Key::Named(NamedKey::Backspace) => {
                        let overlay = match &mut self.state {
                            AppState::Capturing(o) => o,
                            _ => return,
                        };
                        if let Some(idx) = overlay.selected_index {
                            if idx < overlay.annotations.len() {
                                overlay.delete_annotation(idx);
                                overlay.selected_index = None;
                                overlay.sync_step_counter();
                                self.needs_redraw = true;
                            }
                        }
                    }
                    Key::Named(NamedKey::Enter) if overlay.selection.is_some() => {
                        // Quick copy — same path as Ctrl+C / the Copy button,
                        // annotations included, clipboard work off-thread.
                        self.do_copy();
                        return;
                    }
                    Key::Character(ch) => {
                        let ctrl = self.modifiers.control_key();
                        let shift = self.modifiers.shift_key();
                        // Compare case-insensitively so Caps Lock / Shift don't
                        // break shortcuts ("Z" vs "z"); redo is decided by the
                        // actual Shift modifier, not the produced character.
                        let key_lower = ch.as_str().to_lowercase();
                        match key_lower.as_str() {
                            "c" if ctrl => {
                                self.do_copy();
                                return;
                            }
                            "s" if ctrl => {
                                self.do_save();
                                return;
                            }
                            "z" if ctrl && shift => {
                                // Redo
                                let overlay = match &mut self.state {
                                    AppState::Capturing(o) => o,
                                    _ => return,
                                };
                                if apply_redo(
                                    &mut overlay.annotations,
                                    &mut overlay.undo_stack,
                                    &mut overlay.redo_stack,
                                ) {
                                    overlay.selected_index = None;
                                    overlay.sync_step_counter();
                                    self.needs_redraw = true;
                                }
                            }
                            "z" if ctrl => {
                                // Undo
                                let overlay = match &mut self.state {
                                    AppState::Capturing(o) => o,
                                    _ => return,
                                };
                                if apply_undo(
                                    &mut overlay.annotations,
                                    &mut overlay.undo_stack,
                                    &mut overlay.redo_stack,
                                ) {
                                    overlay.selected_index = None;
                                    overlay.sync_step_counter();
                                    self.needs_redraw = true;
                                }
                            }
                            "a" if ctrl => {
                                // Ctrl+A: select entire screen
                                let overlay = match &mut self.state {
                                    AppState::Capturing(o) => o,
                                    _ => return,
                                };
                                overlay.selection = Some(Selection {
                                    x: 0.0,
                                    y: 0.0,
                                    width: overlay.screenshot.width as f32,
                                    height: overlay.screenshot.height as f32,
                                });
                                self.needs_redraw = true;
                            }
                            // Tool keyboard shortcuts (no Ctrl) — config-driven,
                            // case-insensitive so Caps Lock doesn't disable them.
                            key_str if !ctrl => {
                                let shortcuts = &self.config.shortcuts;
                                let eq = |bound: &str| key_str.eq_ignore_ascii_case(bound);
                                let new_tool = if eq(&shortcuts.select) {
                                    Some(ToolKind::Select)
                                } else if eq(&shortcuts.arrow) {
                                    Some(ToolKind::Arrow)
                                } else if eq(&shortcuts.rectangle) {
                                    Some(ToolKind::Rectangle)
                                } else if eq(&shortcuts.circle) {
                                    Some(ToolKind::Circle)
                                } else if eq(&shortcuts.rounded_rect) {
                                    Some(ToolKind::RoundedRect)
                                } else if eq(&shortcuts.line) {
                                    Some(ToolKind::Line)
                                } else if eq(&shortcuts.pencil) {
                                    Some(ToolKind::Pencil)
                                } else if eq(&shortcuts.highlight) {
                                    Some(ToolKind::Highlight)
                                } else if eq(&shortcuts.spotlight) {
                                    Some(ToolKind::Spotlight)
                                } else if eq(&shortcuts.text) {
                                    Some(ToolKind::Text)
                                } else if eq(&shortcuts.pixelate) {
                                    Some(ToolKind::Pixelate)
                                } else if eq(&shortcuts.step_marker) {
                                    Some(ToolKind::StepMarker)
                                } else if eq(&shortcuts.eyedropper) {
                                    Some(ToolKind::Eyedropper)
                                } else if eq(&shortcuts.measurement) {
                                    Some(ToolKind::Measurement)
                                } else {
                                    None
                                };

                                if let Some(tool) = new_tool {
                                    let overlay = match &mut self.state {
                                        AppState::Capturing(o) => o,
                                        _ => return,
                                    };
                                    overlay.active_tool = tool;
                                    overlay.selected_index = None;
                                    self.needs_redraw = true;
                                }
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                self.editor_pointer = (position.x, position.y);
                let pos = if self.is_preview {
                    let window = self.overlay_window.as_ref().unwrap();
                    let size = window.inner_size();
                    self.editor_view.framed_point(
                        position.x,
                        position.y,
                        overlay.screenshot.width,
                        overlay.screenshot.height,
                        size.width,
                        size.height,
                        window.scale_factor(),
                    )
                } else {
                    Point::new(position.x as f32, position.y as f32)
                };
                let prev = overlay.last_mouse_pos;
                overlay.last_mouse_pos = pos;
                let dx = pos.x - prev.x;
                let dy = pos.y - prev.y;

                // Window capture mode: highlight the window under the cursor
                if self.window_capture_mode {
                    let x_off = overlay.screenshot.x_offset;
                    let y_off = overlay.screenshot.y_offset;
                    let screen_x = pos.x as i32 + x_off;
                    let screen_y = pos.y as i32 + y_off;

                    if let Some((wx, wy, ww, wh)) =
                        window_detect::window_at_point(&self.window_rects, screen_x, screen_y)
                    {
                        let sel_x = (wx - x_off) as f32;
                        let sel_y = (wy - y_off) as f32;
                        overlay.selection = Some(Selection {
                            x: sel_x,
                            y: sel_y,
                            width: ww as f32,
                            height: wh as f32,
                        });
                    } else {
                        overlay.selection = None;
                    }
                    self.needs_redraw = true;

                    // Update cursor
                    if let Some(ref window) = self.overlay_window {
                        window.set_cursor(CursorIcon::Crosshair);
                    }
                    return;
                }

                if overlay.is_selecting {
                    // Update selection while dragging
                    if let Some(start) = overlay.drag_start {
                        overlay.selection = Some(Selection::from_points(start, pos));
                        self.needs_redraw = true;
                    }
                } else if let Some(zone) = overlay.drag_zone {
                    // Resizing or moving selection
                    if let Some(ref mut sel) = overlay.selection {
                        if zone == HitZone::Inside {
                            sel.move_by(dx, dy);
                        } else {
                            sel.resize(zone, dx, dy);
                        }
                        self.needs_redraw = true;
                    }
                } else {
                    // Forward to active tool if drawing
                    match overlay.active_tool {
                        ToolKind::Select => {
                            // Resize drag takes priority
                            if let (Some(idx), Some(handle)) =
                                (overlay.selected_index, overlay.resize_handle)
                            {
                                if let Some(ann) = overlay.annotations.get_mut(idx) {
                                    resize_annotation(ann, handle, pos);
                                }
                                self.needs_redraw = true;
                            } else if let (Some(idx), Some(drag_start)) =
                                (overlay.selected_index, overlay.select_drag_start)
                            {
                                let dx = pos.x - drag_start.x;
                                let dy = pos.y - drag_start.y;
                                if let Some(ann) = overlay.annotations.get_mut(idx) {
                                    move_annotation(ann, dx, dy);
                                }
                                overlay.select_drag_start = Some(pos);
                                self.needs_redraw = true;
                            }
                        }
                        ToolKind::Arrow => {
                            if overlay.arrow_tool.is_drawing() {
                                overlay.arrow_tool.on_mouse_move(pos);
                                self.needs_redraw = true;
                            }
                        }
                        ToolKind::Rectangle => {
                            if overlay.rectangle_tool.is_drawing() {
                                overlay.rectangle_tool.on_mouse_move(pos);
                                self.needs_redraw = true;
                            }
                        }
                        ToolKind::RoundedRect => {
                            if overlay.rounded_rect_tool.is_drawing() {
                                overlay.rounded_rect_tool.on_mouse_move(pos);
                                self.needs_redraw = true;
                            }
                        }
                        ToolKind::Circle => {
                            if overlay.circle_tool.is_drawing() {
                                overlay.circle_tool.on_mouse_move(pos);
                                self.needs_redraw = true;
                            }
                        }
                        ToolKind::Line => {
                            if overlay.line_tool.is_drawing() {
                                overlay.line_tool.on_mouse_move(pos);
                                self.needs_redraw = true;
                            }
                        }
                        ToolKind::Measurement => {
                            if overlay.measurement_tool.is_drawing() {
                                overlay.measurement_tool.on_mouse_move(pos);
                                self.needs_redraw = true;
                            }
                        }
                        ToolKind::Pencil => {
                            if overlay.pencil_tool.is_drawing() {
                                overlay.pencil_tool.on_mouse_move(pos);
                                self.needs_redraw = true;
                            }
                        }
                        ToolKind::Highlight => {
                            if overlay.highlight_tool.is_drawing() {
                                overlay.highlight_tool.on_mouse_move(pos);
                                self.needs_redraw = true;
                            }
                        }
                        ToolKind::Spotlight => {
                            if overlay.spotlight_tool.is_drawing() {
                                overlay.spotlight_tool.on_mouse_move(pos);
                                self.needs_redraw = true;
                            }
                        }
                        ToolKind::Text => {
                            // Text tool does not use mouse move
                            overlay.text_tool.on_mouse_move(pos);
                        }
                        ToolKind::Pixelate => {
                            if overlay.pixelate_tool.is_drawing() {
                                overlay.pixelate_tool.on_mouse_move(pos);
                                self.needs_redraw = true;
                            }
                        }
                        ToolKind::StepMarker => {
                            overlay.step_marker_tool.on_mouse_move(pos);
                        }
                        ToolKind::Eyedropper => {
                            // Read color from screenshot pixels at cursor position
                            let px = pos.x as u32;
                            let py = pos.y as u32;
                            if px < overlay.screenshot.width && py < overlay.screenshot.height {
                                let idx = ((py * overlay.screenshot.width + px) * 4) as usize;
                                if idx + 3 < overlay.screenshot.pixels.len() {
                                    let r = overlay.screenshot.pixels[idx] as f32 / 255.0;
                                    let g = overlay.screenshot.pixels[idx + 1] as f32 / 255.0;
                                    let b = overlay.screenshot.pixels[idx + 2] as f32 / 255.0;
                                    overlay.eyedropper_preview = Some(Color::new(r, g, b, 1.0));
                                }
                            }
                            self.needs_redraw = true;
                        }
                    }
                }

                // Update cursor icon based on current state
                if let Some(ref window) = self.overlay_window {
                    let cursor = determine_cursor(overlay, pos, self.is_preview, hit_slop);
                    window.set_cursor(cursor);
                }

                // Always redraw on mouse move for tooltips and cursor feedback
                // (60fps cap prevents this from being wasteful)
                self.needs_redraw = true;
            }

            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: WinitMouseButton::Left,
                ..
            } => {
                // Window capture mode: click confirms the highlighted window
                if self.window_capture_mode {
                    // Selection is already set to the window under cursor
                    self.window_capture_mode = false;
                    self.window_rects.clear();
                    self.needs_redraw = true;
                    tracing::info!("Window captured");
                    return;
                }

                let pos = overlay.last_mouse_pos;

                // 1. Check toolbar hit first (only if selection exists)
                if let Some(ref sel) = overlay.selection.filter(|_| !self.is_preview) {
                    let visible_count = overlay.visible_buttons.len();
                    let toolbar = overlay.toolbar(sel);
                    if let Some(vis_idx) = toolbar.hit_test_dynamic(pos, visible_count) {
                        let btn = overlay.visible_buttons[vis_idx];
                        let action = BUTTONS.get(btn).map(|d| d.action);
                        match action {
                            Some(ButtonAction::Tool(kind)) => {
                                overlay.active_tool = kind;
                                if kind != ToolKind::Select {
                                    overlay.selected_index = None;
                                }
                                self.needs_redraw = true;
                            }
                            Some(ButtonAction::Color(idx)) => {
                                let presets = Color::presets();
                                if idx < presets.len() {
                                    // If an annotation is selected, recolor it
                                    if let Some(sel_idx) = overlay.selected_index {
                                        if sel_idx < overlay.annotations.len() {
                                            let old_ann = overlay.annotations[sel_idx].clone();
                                            recolor_annotation(
                                                &mut overlay.annotations[sel_idx],
                                                presets[idx],
                                            );
                                            record_undo(
                                                &mut overlay.undo_stack,
                                                &mut overlay.redo_stack,
                                                UndoAction::Modify(sel_idx, old_ann),
                                            );
                                        }
                                    } else {
                                        overlay.set_color_all(presets[idx]);
                                    }
                                    self.needs_redraw = true;
                                }
                            }
                            Some(ButtonAction::Ocr) => {
                                self.do_ocr();
                                return;
                            }
                            Some(ButtonAction::Upload) => {
                                self.do_upload(_event_loop, false);
                                return;
                            }
                            Some(ButtonAction::Pin) => {
                                self.do_pin(_event_loop);
                                return;
                            }
                            Some(ButtonAction::Copy) => {
                                self.do_copy();
                                return;
                            }
                            Some(ButtonAction::Save) => {
                                self.do_save();
                                return;
                            }
                            None => {}
                        }
                        return;
                    }
                }

                // The preview edits a fixed cropped image. Its padding is not
                // a new capture surface and its edges cannot resize the crop.
                if self.is_preview && overlay.selection_hit(pos, true).is_none() {
                    return;
                }

                // 2. No selection yet — start selecting
                if overlay.selection.is_none() {
                    overlay.is_selecting = true;
                    overlay.drag_start = Some(pos);
                    return;
                }

                // 3. Selection exists — hit-test it
                if overlay.selection.is_some() {
                    match overlay.selection_hit(pos, self.is_preview) {
                        Some(HitZone::Inside) => {
                            // Start annotation with active tool
                            match overlay.active_tool {
                                ToolKind::Select => {
                                    // Check resize handles first (if an annotation is selected)
                                    if let Some(idx) = overlay.selected_index {
                                        if let Some(ann) = overlay.annotations.get(idx) {
                                            if let Some((bx, by, bw, bh)) =
                                                annotation_bounding_box(ann)
                                            {
                                                let handles = [
                                                    (Point::new(bx, by), ResizeHandle::TopLeft),
                                                    (
                                                        Point::new(bx + bw, by),
                                                        ResizeHandle::TopRight,
                                                    ),
                                                    (
                                                        Point::new(bx, by + bh),
                                                        ResizeHandle::BottomLeft,
                                                    ),
                                                    (
                                                        Point::new(bx + bw, by + bh),
                                                        ResizeHandle::BottomRight,
                                                    ),
                                                ];
                                                for (hp, handle) in &handles {
                                                    if (pos.x - hp.x).abs() < hit_slop
                                                        && (pos.y - hp.y).abs() < hit_slop
                                                    {
                                                        overlay.resize_handle = Some(*handle);
                                                        // Snapshot annotation before resize for undo
                                                        overlay.pre_drag_annotation = Some((
                                                            idx,
                                                            overlay.annotations[idx].clone(),
                                                        ));
                                                        self.needs_redraw = true;
                                                        return;
                                                    }
                                                }
                                            }
                                        }
                                    }

                                    // Check annotations in REVERSE order (top-most first)
                                    let mut found = None;
                                    for (idx, ann) in overlay.annotations.iter().enumerate().rev() {
                                        if hit_test_annotation(ann, &pos, hit_slop) {
                                            found = Some(idx);
                                            break;
                                        }
                                    }
                                    if let Some(idx) = found {
                                        // If clicking an already-selected Text annotation, re-enter edit mode
                                        if overlay.selected_index == Some(idx) {
                                            if let Some(Annotation::Text {
                                                position,
                                                text,
                                                color,
                                                font_size,
                                            }) = overlay.annotations.get(idx).cloned()
                                            {
                                                // Take the annotation out and remember it: committing
                                                // records ONE Modify undo step; Escape restores it.
                                                let removed = overlay.annotations.remove(idx);
                                                overlay.text_edit_origin = Some((idx, removed));
                                                overlay.selected_index = None;
                                                overlay.text_input_active = true;
                                                overlay.text_input_position = position;
                                                overlay.text_input_buffer = text;
                                                overlay.text_input_font_size = font_size;
                                                overlay.current_color = color;
                                                self.needs_redraw = true;
                                                return;
                                            }
                                        }
                                        overlay.selected_index = Some(idx);
                                        overlay.select_drag_start = Some(pos);
                                        // Snapshot annotation before move for undo
                                        overlay.pre_drag_annotation =
                                            Some((idx, overlay.annotations[idx].clone()));
                                    } else {
                                        overlay.selected_index = None;
                                    }
                                }
                                ToolKind::Arrow => overlay.arrow_tool.on_mouse_down(pos),
                                ToolKind::Rectangle => overlay.rectangle_tool.on_mouse_down(pos),
                                ToolKind::RoundedRect => {
                                    overlay.rounded_rect_tool.on_mouse_down(pos)
                                }
                                ToolKind::Circle => overlay.circle_tool.on_mouse_down(pos),
                                ToolKind::Line => overlay.line_tool.on_mouse_down(pos),
                                ToolKind::Measurement => {
                                    overlay.measurement_tool.on_mouse_down(pos)
                                }
                                ToolKind::Pencil => overlay.pencil_tool.on_mouse_down(pos),
                                ToolKind::Highlight => overlay.highlight_tool.on_mouse_down(pos),
                                ToolKind::Text => {
                                    overlay.text_tool.on_mouse_down(pos);
                                    if let Some(p) = overlay.text_tool.take_pending_position() {
                                        overlay.text_input_active = true;
                                        overlay.text_input_position = p;
                                        overlay.text_input_buffer.clear();
                                        overlay.text_input_font_size =
                                            overlay.text_tool.font_size();
                                    }
                                }
                                ToolKind::Pixelate => overlay.pixelate_tool.on_mouse_down(pos),
                                ToolKind::StepMarker => overlay.step_marker_tool.on_mouse_down(pos),
                                ToolKind::Spotlight => overlay.spotlight_tool.on_mouse_down(pos),
                                ToolKind::Eyedropper => {
                                    if let Some(color) = overlay.eyedropper_preview {
                                        overlay.set_color_all(color);
                                        overlay.active_tool = ToolKind::Arrow;
                                    }
                                }
                            }
                            self.needs_redraw = true;
                        }
                        Some(zone) => {
                            // Start resize
                            overlay.drag_zone = Some(zone);
                        }
                        None => {
                            // Outside selection — clear and start new
                            overlay.selection = None;
                            overlay.is_selecting = true;
                            overlay.drag_start = Some(pos);
                            self.needs_redraw = true;
                        }
                    }
                }
            }

            WindowEvent::MouseInput {
                state: ElementState::Released,
                button: WinitMouseButton::Left,
                ..
            } => {
                let pos = overlay.last_mouse_pos;

                if overlay.is_selecting {
                    overlay.is_selecting = false;
                    // Finalize selection — ensure minimum size
                    if let Some(start) = overlay.drag_start.take() {
                        let sel = Selection::from_points(start, pos);
                        if sel.width > 2.0 && sel.height > 2.0 {
                            overlay.selection = Some(sel);
                        }
                    }
                    self.needs_redraw = true;
                } else if overlay.drag_zone.is_some() {
                    overlay.drag_zone = None;
                } else {
                    // Finalize annotation
                    let annotation = match overlay.active_tool {
                        ToolKind::Select => {
                            // Record undo for move/resize if annotation changed
                            if let Some((idx, old_ann)) = overlay.pre_drag_annotation.take() {
                                if idx < overlay.annotations.len()
                                    && overlay.annotations[idx] != old_ann
                                {
                                    record_undo(
                                        &mut overlay.undo_stack,
                                        &mut overlay.redo_stack,
                                        UndoAction::Modify(idx, old_ann),
                                    );
                                }
                            }
                            overlay.select_drag_start = None;
                            overlay.resize_handle = None;
                            None
                        }
                        ToolKind::Arrow => overlay.arrow_tool.on_mouse_up(pos),
                        ToolKind::Rectangle => overlay.rectangle_tool.on_mouse_up(pos),
                        ToolKind::RoundedRect => overlay.rounded_rect_tool.on_mouse_up(pos),
                        ToolKind::Circle => overlay.circle_tool.on_mouse_up(pos),
                        ToolKind::Line => overlay.line_tool.on_mouse_up(pos),
                        ToolKind::Measurement => overlay.measurement_tool.on_mouse_up(pos),
                        ToolKind::Pencil => overlay.pencil_tool.on_mouse_up(pos),
                        ToolKind::Highlight => overlay.highlight_tool.on_mouse_up(pos),
                        ToolKind::Text => overlay.text_tool.on_mouse_up(pos),
                        ToolKind::Pixelate => overlay.pixelate_tool.on_mouse_up(pos),
                        ToolKind::StepMarker => overlay.step_marker_tool.on_mouse_up(pos),
                        ToolKind::Spotlight => overlay.spotlight_tool.on_mouse_up(pos),
                        ToolKind::Eyedropper => None,
                    };
                    if let Some(ann) = annotation {
                        overlay.annotations.push(ann);
                        record_undo(
                            &mut overlay.undo_stack,
                            &mut overlay.redo_stack,
                            UndoAction::Add(overlay.annotations.len() - 1),
                        );
                        self.needs_redraw = true;
                    }
                }
            }

            // Right-click: color picker on swatch, otherwise close overlay
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: WinitMouseButton::Right,
                ..
            } => {
                if let AppState::Capturing(ref mut overlay) = self.state {
                    if let Some(ref sel) = overlay.selection {
                        let visible_count = overlay.visible_buttons.len();
                        let toolbar = overlay.toolbar(sel);
                        let pos = overlay.last_mouse_pos;
                        if let Some(vis_idx) = toolbar.hit_test_dynamic(pos, visible_count) {
                            let btn = overlay.visible_buttons[vis_idx];
                            if let Some(ButtonAction::Color(swatch_idx)) =
                                BUTTONS.get(btn).map(|d| d.action)
                            {
                                let current = Color::presets()
                                    .get(swatch_idx)
                                    .copied()
                                    .unwrap_or_else(Color::red);
                                if let Some(ref w) = self.overlay_window {
                                    w.set_visible(false);
                                }
                                let picked = snipping_tool_ul::color_picker::pick_color(&current);
                                if let Some(ref w) = self.overlay_window {
                                    w.set_visible(true);
                                }
                                if let Some(new_color) = picked {
                                    overlay.set_color_all(new_color);
                                    if let Some(idx) = overlay.selected_index {
                                        if idx < overlay.annotations.len() {
                                            let old_ann = overlay.annotations[idx].clone();
                                            recolor_annotation(
                                                &mut overlay.annotations[idx],
                                                new_color,
                                            );
                                            record_undo(
                                                &mut overlay.undo_stack,
                                                &mut overlay.redo_stack,
                                                UndoAction::Modify(idx, old_ann),
                                            );
                                        }
                                    }
                                    self.needs_redraw = true;
                                }
                                return;
                            }
                        }
                    }
                }
                if let AppState::Capturing(ref overlay) = self.state {
                    if overlay.selection.is_some() {
                        return;
                    }
                }
                self.close_overlay();
                return;
            }

            WindowEvent::MouseWheel { delta, .. } => {
                let scroll = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(pos) => pos.y as f32 / 20.0,
                };
                if self.is_preview && !self.modifiers.shift_key() {
                    let window = self.overlay_window.as_ref().unwrap();
                    let size = window.inner_size();
                    self.editor_view.zoom_framed(
                        self.editor_pointer.0,
                        self.editor_pointer.1,
                        scroll as f64,
                        overlay.screenshot.width,
                        overlay.screenshot.height,
                        size.width,
                        size.height,
                        window.scale_factor(),
                    );
                    overlay.last_mouse_pos = self.editor_view.framed_point(
                        self.editor_pointer.0,
                        self.editor_pointer.1,
                        overlay.screenshot.width,
                        overlay.screenshot.height,
                        size.width,
                        size.height,
                        window.scale_factor(),
                    );
                } else if overlay.text_input_active {
                    // Shift-wheel adjusts the annotation, ordinary wheel zooms.
                    overlay.text_input_font_size =
                        (overlay.text_input_font_size + scroll).clamp(10.0, 72.0);
                } else if overlay.active_tool == ToolKind::StepMarker {
                    let new_size = overlay.step_marker_tool.size() + scroll * 2.0;
                    overlay.step_marker_tool.set_size(new_size);
                } else if overlay.active_tool == ToolKind::RoundedRect {
                    let new_radius = overlay.rounded_rect_tool.radius() + scroll * 2.0;
                    overlay.rounded_rect_tool.set_radius(new_radius);
                } else {
                    overlay.set_thickness_all(overlay.current_thickness + scroll);
                }
                self.needs_redraw = true;
            }

            _ => {}
        }

        if self.needs_redraw {
            if let Some(w) = &self.overlay_window {
                w.request_redraw();
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.process_tray_events(event_loop);

        // Drain ALL pending hotkey events, not just one per wakeup
        while let Ok(event) = global_hotkey::GlobalHotKeyEvent::receiver().try_recv() {
            if event.state == global_hotkey::HotKeyState::Pressed
                && Some(event.id) == self.hotkey_id
            {
                tracing::info!("Global hotkey pressed — triggering capture");
                self.trigger_capture(event_loop);
            }
        }

        // Apply async folder-picker results for the settings window.
        if let Some(ref mut sw) = self.settings_window {
            if sw.poll_browse() {
                sw.window.request_redraw();
            }
        }

        // Update countdown overlay
        if let Some(ref mut cd) = self.countdown {
            let now = Instant::now();
            if cd.tick(now) {
                // Hide before dropping so the window is gone from the screen
                // before the screenshot is taken.
                cd.hide();
                self.countdown = None;
                // Schedule capture 300ms from now — gives the OS time to
                // fully remove the countdown window from the screen
                self.capture_at = Some(now + Duration::from_millis(300));
            } else {
                event_loop.set_control_flow(ControlFlow::WaitUntil(cd.next_tick()));
            }
        }

        if let Some(capture_time) = self.capture_at {
            if Instant::now() >= capture_time {
                self.capture_at = None;
                self.trigger_capture(event_loop);
            } else {
                event_loop.set_control_flow(ControlFlow::WaitUntil(capture_time));
            }
        } else if !self.needs_redraw {
            // Wake exactly at toast expiry to clear it — don't re-render before
            if let AppState::Capturing(ref overlay) = self.state {
                if let Some(until) = overlay.toast_until {
                    if Instant::now() >= until {
                        self.needs_redraw = true; // expired — render to clear it
                    } else {
                        event_loop.set_control_flow(ControlFlow::WaitUntil(until));
                    }
                } else {
                    event_loop.set_control_flow(ControlFlow::Wait);
                }
            } else {
                event_loop.set_control_flow(ControlFlow::Wait);
            }
        }

        // Frame rate cap: only render if enough time has passed since last frame
        if self.needs_redraw {
            let now = Instant::now();
            let elapsed = now.duration_since(self.last_render);
            if elapsed >= FRAME_INTERVAL {
                self.render();
                self.last_render = now;
            } else {
                // Schedule wake-up for next frame
                let next_frame = self.last_render + FRAME_INTERVAL;
                event_loop.set_control_flow(ControlFlow::WaitUntil(next_frame));
            }
        }

        // While a folder-picker dialog is open we must self-schedule wakeups —
        // the dialog produces no winit events. This is set LAST so the idle
        // ControlFlow::Wait above can't overwrite it (last write wins).
        if self
            .settings_window
            .as_ref()
            .is_some_and(|sw| sw.browse_pending())
        {
            event_loop.set_control_flow(ControlFlow::WaitUntil(
                Instant::now() + Duration::from_millis(100),
            ));
        }

        self.trim_when_tray_idle();
    }
}

impl App {
    /// Once every window has closed, hand untouched pages (fonts, freed
    /// capture buffers) back to Windows so the tray process stays small.
    /// Runs once per idle period, never on a timer.
    fn trim_when_tray_idle(&mut self) {
        let idle = matches!(self.state, AppState::Idle)
            && self.launcher.is_none()
            && self.settings_window.is_none()
            && self.history_window.is_none()
            && self.countdown.is_none()
            && self.pinned_windows.is_empty()
            && self.overlay_window.is_none()
            && !self.picker_active
            && self.capture_at.is_none();
        if !idle {
            self.idle_trimmed = false;
            return;
        }
        if self.idle_trimmed {
            return;
        }
        self.idle_trimmed = true;
        #[cfg(target_os = "windows")]
        unsafe {
            use windows::Win32::System::{
                ProcessStatus::EmptyWorkingSet, Threading::GetCurrentProcess,
            };
            let _ = EmptyWorkingSet(GetCurrentProcess());
        }
    }
}

fn run_tray_app(config: Config, show_home: bool) {
    let event_loop = match EventLoop::<UserEvent>::with_user_event().build() {
        Ok(el) => el,
        Err(e) => {
            snipping_tool_ul::installer::show_error(&format!("Failed to start: {e}"));
            std::process::exit(1);
        }
    };
    let proxy = event_loop.create_proxy();
    let activation = proxy.clone();
    let _instance = match snipping_tool_ul::instance::acquire(show_home, move || {
        let _ = activation.send_event(UserEvent::ShowHome);
    }) {
        Ok(Some(instance)) => instance,
        Ok(None) => return,
        Err(e) => {
            snipping_tool_ul::installer::show_error(&e);
            return;
        }
    };
    // Native tray/hotkey callbacks wake the sleeping event loop. Idle needs no timer.
    let hotkey_proxy = proxy.clone();
    global_hotkey::GlobalHotKeyEvent::set_event_handler(Some(move |event| {
        let _ = hotkey_proxy.send_event(UserEvent::Hotkey(event));
    }));
    let tray_proxy = proxy.clone();
    TrayIconEvent::set_event_handler(Some(move |event| {
        let _ = tray_proxy.send_event(UserEvent::Tray(event));
    }));
    let menu_proxy = proxy.clone();
    MenuEvent::set_event_handler(Some(move |event| {
        let _ = menu_proxy.send_event(UserEvent::Menu(event));
    }));
    let mut app = App::new(config, proxy);
    app.show_home_on_start = show_home;
    if let Err(e) = event_loop.run_app(&mut app) {
        snipping_tool_ul::installer::show_error(&format!("Event loop error: {e}"));
        std::process::exit(1);
    }
}

fn run_tray_app_with_immediate_capture(config: Config) {
    let event_loop = match EventLoop::<UserEvent>::with_user_event().build() {
        Ok(el) => el,
        Err(e) => {
            snipping_tool_ul::installer::show_error(&format!("Failed to start: {e}"));
            std::process::exit(1);
        }
    };
    let mut app = App::new(config, event_loop.create_proxy());
    app.immediate_capture = true;
    app.cli_only = true;
    if let Err(e) = event_loop.run_app(&mut app) {
        snipping_tool_ul::installer::show_error(&format!("Event loop error: {e}"));
        std::process::exit(1);
    }
}

fn run_cli_capture(
    clipboard: bool,
    save: Option<String>,
    delay: u64,
    region: Option<capture::ScreenRect>,
) {
    if delay > 0 {
        tracing::info!("Waiting {} seconds...", delay);
        std::thread::sleep(std::time::Duration::from_secs(delay));
    }

    // If neither --clipboard nor --save: open interactive overlay
    if !clipboard && save.is_none() && region.is_none() {
        tracing::info!("Opening interactive capture...");
        let config = Config::load();
        run_tray_app_with_immediate_capture(config);
        return;
    }

    // Non-interactive: capture full screen directly
    let capturer = match capture::create_capturer() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to create capturer: {}", e);
            std::process::exit(1);
        }
    };

    #[cfg(target_os = "windows")]
    let capture_result = match region {
        Some(rect) => capture::capture_region(rect).map(|screen| vec![screen]),
        None => capturer.capture_all_screens(),
    };
    #[cfg(not(target_os = "windows"))]
    let capture_result = if region.is_some() {
        Err(capture::CaptureError::PlatformError(
            "--region is supported on Windows only".to_string(),
        ))
    } else {
        capturer.capture_all_screens()
    };
    let screens = match capture_result {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Capture failed: {}", e);
            std::process::exit(1);
        }
    };

    if screens.is_empty() {
        eprintln!("No screens captured");
        std::process::exit(1);
    }

    let screen = &screens[0];
    tracing::info!(
        "CLI capture: {}x{} physical pixels at ({},{})",
        screen.width,
        screen.height,
        screen.x_offset,
        screen.y_offset
    );

    // --clipboard and --save are independent; both can be given at once.
    if clipboard {
        match export::copy_to_clipboard(&screen.pixels, screen.width, screen.height) {
            Ok(_) => {
                println!(
                    "Copied {}x{} screenshot to clipboard",
                    screen.width, screen.height
                );
            }
            Err(e) => {
                eprintln!("Clipboard error: {}", e);
                std::process::exit(1);
            }
        }
    }
    if let Some(path) = save {
        let img =
            match image::RgbaImage::from_raw(screen.width, screen.height, screen.pixels.clone()) {
                Some(img) => img,
                None => {
                    eprintln!("Invalid image data ({}x{})", screen.width, screen.height);
                    std::process::exit(1);
                }
            };
        match img.save(&path) {
            Ok(_) => println!("Saved to {}", path),
            Err(e) => {
                eprintln!("Save error: {}", e);
                std::process::exit(1);
            }
        }
    }
}

fn main() {
    #[cfg(target_os = "windows")]
    capture::initialize_dpi();
    tracing_subscriber::fmt::init();
    tracing::info!("Snipping Tool UL starting");

    // Set Windows Application User Model ID so toast notifications display
    #[cfg(target_os = "windows")]
    {
        use windows::core::w;
        use windows::Win32::UI::Shell::SetCurrentProcessExplicitAppUserModelID;
        unsafe {
            let _ = SetCurrentProcessExplicitAppUserModelID(w!("SnippingToolUL.App"));
        }
    }

    let cli = Cli::parse();

    // For CLI subcommands, attach to the parent console so println!/eprintln!
    // produce visible output (windows_subsystem = "windows" detaches stdout).
    #[cfg(target_os = "windows")]
    if cli.command.is_some() {
        use windows::Win32::System::Console::{AttachConsole, ATTACH_PARENT_PROCESS};
        unsafe {
            let _ = AttachConsole(ATTACH_PARENT_PROCESS);
        }
    }

    match cli.command {
        None => {
            // This app is distributed as a portable build.
            let config = Config::load();
            tracing::info!(
                "Config loaded: hotkey={}, color={}, thickness={}",
                config.hotkey.capture,
                config.general.default_color,
                config.general.default_thickness
            );
            run_tray_app(config, !cli.background);
        }
        Some(Commands::Capture {
            clipboard,
            save,
            delay,
            region,
        }) => {
            run_cli_capture(
                clipboard || (region.is_some() && save.is_none()),
                save,
                delay,
                region,
            );
        }
        Some(Commands::Install {
            apply,
            replace_windows_snipping_tool,
            interactive,
        }) => {
            let result = std::env::current_exe()
                .map_err(|e| e.to_string())
                .and_then(|exe| {
                    snipping_tool_ul::windows_integration::install(
                        &exe,
                        replace_windows_snipping_tool,
                        apply,
                        interactive,
                    )
                });
            match result {
                Ok(message) => println!("{message}"),
                Err(e) => {
                    eprintln!("{e}");
                    std::process::exit(1);
                }
            }
        }
        Some(Commands::Uninstall) => {
            eprintln!("Snipping Tool UL is portable. Install/uninstall commands are disabled to protect upstream installations.");
            std::process::exit(1);
        }
    }
}
