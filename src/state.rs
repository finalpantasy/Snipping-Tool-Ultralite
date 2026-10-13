use crate::capture::CapturedScreen;
use crate::geometry::{Color, Point};
use crate::icons::IconCache;
use crate::overlay::selection::{HitZone, Selection};
use crate::tools::arrow::ArrowTool;
use crate::tools::circle::CircleTool;
use crate::tools::highlight::HighlightTool;
use crate::tools::line::LineTool;
use crate::tools::measurement::MeasurementTool;
use crate::tools::pencil::PencilTool;
use crate::tools::pixelate::PixelateTool;
use crate::tools::rectangle::RectangleTool;
use crate::tools::rounded_rect::RoundedRectTool;
use crate::tools::spotlight::SpotlightTool;
use crate::tools::step_marker::StepMarkerTool;
use crate::tools::text::TextTool;
use crate::tools::{Annotation, ResizeHandle, ToolKind, UndoAction};

pub enum AppState {
    Idle,
    Capturing(Box<OverlayState>),
}

pub struct OverlayState {
    pub screenshot: CapturedScreen,
    /// Canvas height used by every toolbar render and input path.
    pub canvas_height: u32,
    pub preview_mode: bool,
    pub selection: Option<Selection>,
    pub annotations: Vec<Annotation>,
    pub undo_stack: Vec<UndoAction>,
    pub redo_stack: Vec<UndoAction>,
    /// Snapshot of an annotation before a drag/resize starts, for undo recording.
    pub pre_drag_annotation: Option<(usize, Annotation)>,
    pub active_tool: ToolKind,
    pub arrow_tool: ArrowTool,
    pub rectangle_tool: RectangleTool,
    pub rounded_rect_tool: RoundedRectTool,
    pub circle_tool: CircleTool,
    pub line_tool: LineTool,
    pub pencil_tool: PencilTool,
    pub highlight_tool: HighlightTool,
    pub text_tool: TextTool,
    pub pixelate_tool: PixelateTool,
    pub step_marker_tool: StepMarkerTool,
    pub spotlight_tool: SpotlightTool,
    pub measurement_tool: MeasurementTool,
    pub text_input_active: bool,
    pub text_input_buffer: String,
    pub text_input_position: Point,
    pub text_input_font_size: f32,
    /// When re-editing an existing Text annotation: its original index and
    /// content, so committing records a single Modify undo step (and Escape
    /// restores it) instead of a Delete + Add pair.
    pub text_edit_origin: Option<(usize, Annotation)>,
    pub current_color: Color,
    pub current_thickness: f32,
    pub is_selecting: bool,
    pub drag_start: Option<Point>,
    pub drag_zone: Option<HitZone>,
    pub last_mouse_pos: Point,
    pub selected_index: Option<usize>,
    pub select_drag_start: Option<Point>,
    pub resize_handle: Option<ResizeHandle>,
    pub icon_cache: IconCache,
    pub eyedropper_preview: Option<Color>,
    /// In-overlay toast notification (visible on top of the overlay)
    pub toast_message: Option<String>,
    pub toast_until: Option<std::time::Instant>,
    /// Visible toolbar button indices (original 0-23 indices), computed from ToolbarConfig.
    pub visible_buttons: Vec<usize>,
}

impl OverlayState {
    /// Commit active text before a toolbar action exports or changes tools.
    pub fn commit_text(&mut self) {
        if !self.text_input_active {
            return;
        }
        let buffer = std::mem::take(&mut self.text_input_buffer);
        let ann = (!buffer.is_empty()).then(|| Annotation::Text {
            position: self.text_input_position,
            text: buffer,
            color: self.current_color,
            font_size: self.text_input_font_size,
        });
        if let Some((idx, old)) = self.text_edit_origin.take() {
            let idx = idx.min(self.annotations.len());
            if let Some(ann) = ann {
                self.annotations.insert(idx, ann);
                crate::tools::record_undo(
                    &mut self.undo_stack,
                    &mut self.redo_stack,
                    UndoAction::Modify(idx, old),
                );
            } else {
                crate::tools::record_undo(
                    &mut self.undo_stack,
                    &mut self.redo_stack,
                    UndoAction::Delete(idx, old),
                );
            }
        } else if let Some(ann) = ann {
            self.annotations.push(ann);
            crate::tools::record_undo(
                &mut self.undo_stack,
                &mut self.redo_stack,
                UndoAction::Add(self.annotations.len() - 1),
            );
        }
        self.text_input_active = false;
    }
    pub fn new(screenshot: CapturedScreen, config: &crate::config::Config) -> Self {
        let color = config.default_color();
        let thickness = config.clamped_thickness();

        Self {
            canvas_height: screenshot.height,
            preview_mode: false,
            screenshot,
            selection: None,
            annotations: Vec::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            pre_drag_annotation: None,
            active_tool: ToolKind::Arrow,
            arrow_tool: ArrowTool::new(color, thickness),
            rectangle_tool: RectangleTool::new(color, thickness),
            rounded_rect_tool: RoundedRectTool::new(color, thickness),
            circle_tool: CircleTool::new(color, thickness),
            line_tool: LineTool::new(color, thickness),
            pencil_tool: PencilTool::new(color, thickness),
            highlight_tool: HighlightTool::new(color),
            text_tool: TextTool::new(color, 20.0),
            pixelate_tool: PixelateTool::new(10),
            step_marker_tool: StepMarkerTool::new(color, 28.0),
            spotlight_tool: SpotlightTool::new(),
            measurement_tool: MeasurementTool::new(color),
            text_input_active: false,
            text_input_buffer: String::new(),
            text_input_position: Point::new(0.0, 0.0),
            text_input_font_size: 20.0,
            text_edit_origin: None,
            current_color: color,
            current_thickness: thickness,
            is_selecting: false,
            drag_start: None,
            drag_zone: None,
            last_mouse_pos: Point::new(0.0, 0.0),
            selected_index: None,
            select_drag_start: None,
            resize_handle: None,
            icon_cache: IconCache::new(),
            eyedropper_preview: None,
            toast_message: None,
            toast_until: None,
            visible_buttons: config.toolbar.visible_button_indices(),
        }
    }

    pub fn toolbar(&self, selection: &Selection) -> crate::overlay::toolbar::Toolbar {
        if self.preview_mode {
            return crate::overlay::toolbar::Toolbar {
                x: 8.,
                y: self.screenshot.height as f32 + 8.,
                width: crate::overlay::toolbar::Toolbar::toolbar_width_dynamic(
                    self.visible_buttons.len(),
                ),
                height: crate::overlay::toolbar::TOOLBAR_HEIGHT,
            };
        }
        crate::overlay::toolbar::Toolbar::position_for_dynamic(
            selection,
            self.canvas_height as f32,
            self.visible_buttons.len(),
        )
    }

    pub fn selection_hit(&self, pos: Point, preview: bool) -> Option<HitZone> {
        if preview {
            (pos.x >= 0.0
                && pos.y >= 0.0
                && pos.x < self.screenshot.width as f32
                && pos.y < self.screenshot.height as f32)
                .then_some(HitZone::Inside)
        } else {
            self.selection
                .as_ref()
                .and_then(|sel| sel.hit_test(pos, 8.0))
        }
    }

    /// Set the drawing color on every tool that has one, plus `current_color`.
    pub fn set_color_all(&mut self, color: Color) {
        self.current_color = color;
        self.arrow_tool.set_color(color);
        self.rectangle_tool.set_color(color);
        self.rounded_rect_tool.set_color(color);
        self.circle_tool.set_color(color);
        self.line_tool.set_color(color);
        self.pencil_tool.set_color(color);
        self.highlight_tool.set_color(color);
        self.text_tool.set_color(color);
        self.step_marker_tool.set_color(color);
        self.measurement_tool.set_color(color);
    }

    /// Set the stroke thickness on every tool that has one, plus `current_thickness`.
    pub fn set_thickness_all(&mut self, thickness: f32) {
        let t = thickness.clamp(1.0, 20.0);
        self.current_thickness = t;
        self.arrow_tool.set_thickness(t);
        self.rectangle_tool.set_thickness(t);
        self.rounded_rect_tool.set_thickness(t);
        self.circle_tool.set_thickness(t);
        self.line_tool.set_thickness(t);
        self.pencil_tool.set_thickness(t);
    }

    /// Resync the step-marker counter to one past the highest number present,
    /// so undo/redo/delete don't leave gaps in the numbering.
    pub fn sync_step_counter(&mut self) {
        let max = self
            .annotations
            .iter()
            .filter_map(|a| match a {
                Annotation::StepMarker { number, .. } => Some(*number),
                _ => None,
            })
            .max()
            .unwrap_or(0);
        self.step_marker_tool.set_next_number(max.saturating_add(1));
    }

    /// Crop the current selection and flatten all annotations into it.
    /// Returns `(pixels, width, height)`, or `None` if there is no selection
    /// or it has zero area.
    pub fn flattened_selection(&self) -> Option<(Vec<u8>, u32, u32)> {
        let sel = self.selection.as_ref()?;
        let r = sel.clamped(self.screenshot.width, self.screenshot.height);
        if r.w == 0 || r.h == 0 {
            return None;
        }
        let pixels = crate::export::crop_and_flatten(
            &self.screenshot.pixels,
            self.screenshot.width,
            r.x,
            r.y,
            r.w,
            r.h,
            &self.annotations,
        );
        Some((pixels, r.w, r.h))
    }

    /// Crop the current selection WITHOUT annotations (raw screenshot pixels).
    pub fn raw_selection(&self) -> Option<(Vec<u8>, u32, u32)> {
        let sel = self.selection.as_ref()?;
        let r = sel.clamped(self.screenshot.width, self.screenshot.height);
        if r.w == 0 || r.h == 0 {
            return None;
        }
        let mut cropped = vec![0u8; (r.w as usize) * (r.h as usize) * 4];
        let src_w = self.screenshot.width;
        for row in 0..r.h {
            let src_row = r.y + row;
            if src_row >= self.screenshot.height {
                break;
            }
            let src_offset = ((src_row * src_w + r.x) * 4) as usize;
            let dst_offset = (row * r.w * 4) as usize;
            let copy_w = (r.w.min(src_w.saturating_sub(r.x)) * 4) as usize;
            let src_end = (src_offset + copy_w).min(self.screenshot.pixels.len());
            if src_end > src_offset {
                let actual = src_end - src_offset;
                cropped[dst_offset..dst_offset + actual]
                    .copy_from_slice(&self.screenshot.pixels[src_offset..src_end]);
            }
        }
        Some((cropped, r.w, r.h))
    }

    /// Show an in-overlay toast for the given duration
    pub fn show_toast(&mut self, message: String, duration_ms: u64) {
        self.toast_message = Some(message);
        self.toast_until =
            Some(std::time::Instant::now() + std::time::Duration::from_millis(duration_ms));
    }

    /// Clear expired toast
    pub fn clear_expired_toast(&mut self) {
        if let Some(until) = self.toast_until {
            if std::time::Instant::now() >= until {
                self.toast_message = None;
                self.toast_until = None;
            }
        }
    }
}
