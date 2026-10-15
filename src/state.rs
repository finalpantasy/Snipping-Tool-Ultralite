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
    /// The annotations last copied and the clipboard sequence number that
    /// copy produced. Another app's copy or a later edit makes it stale.
    pub clipboard_copy: Option<(Vec<Annotation>, u32)>,
}

impl OverlayState {
    /// Adds several annotations as one undo step.
    pub fn add_annotations(&mut self, added: Vec<Annotation>) {
        if added.is_empty() {
            return;
        }
        let first = self.annotations.len();
        self.annotations.extend(added);
        // Highest index first, so undo removes them without shifting the rest.
        let adds = (first..self.annotations.len())
            .rev()
            .map(UndoAction::Add)
            .collect();
        crate::tools::record_undo(
            &mut self.undo_stack,
            &mut self.redo_stack,
            UndoAction::Batch(adds),
        );
    }

    /// Whether the clipboard still holds this image exactly as shown, given
    /// the clipboard's current sequence number. Unfinished text counts as a change.
    pub fn clipboard_current(&self, sequence: u32) -> bool {
        (!self.text_input_active || self.text_input_buffer.is_empty())
            && self
                .clipboard_copy
                .as_ref()
                .is_some_and(|(copied, s)| *s == sequence && *copied == self.annotations)
    }
    /// Selects the topmost annotation under `pos` for the right-click menu.
    /// Pending text is committed first: a text re-edit removes its annotation
    /// from the list, so indices found before the commit would go stale.
    pub fn select_for_context_menu(&mut self, pos: &Point, slop: f32) -> Option<usize> {
        self.commit_text();
        let hit = self
            .annotations
            .iter()
            .rposition(|a| crate::tools::hit_test_annotation(a, pos, slop));
        if hit.is_some() {
            self.selected_index = hit;
        }
        hit
    }
    /// The screenshot color under `pos`, or `None` outside the image. A plain
    /// cast would clamp negative positions onto the first row or column.
    pub fn sample_color(&self, pos: &Point) -> Option<Color> {
        let (w, h) = (self.screenshot.width, self.screenshot.height);
        if !(pos.x >= 0. && pos.y >= 0. && pos.x < w as f32 && pos.y < h as f32) {
            return None;
        }
        let i = ((pos.y as u32 * w + pos.x as u32) * 4) as usize;
        let p = self.screenshot.pixels.get(i..i + 3)?;
        Some(Color::new(
            p[0] as f32 / 255.,
            p[1] as f32 / 255.,
            p[2] as f32 / 255.,
            1.,
        ))
    }
    /// Commit active text before a toolbar action exports or changes tools.
    pub fn commit_text(&mut self) {
        if !self.text_input_active {
            return;
        }
        let buffer = std::mem::take(&mut self.text_input_buffer);
        // Whitespace-only text would commit an invisible annotation and an undo step.
        let ann = (!buffer.trim().is_empty()).then(|| Annotation::Text {
            position: self.text_input_position,
            text: buffer,
            color: self.current_color,
            font_size: self.text_input_font_size,
        });
        if let Some((idx, old)) = self.text_edit_origin.take() {
            let idx = idx.min(self.annotations.len());
            if let Some(ann) = ann {
                let changed = ann != old;
                self.annotations.insert(idx, ann);
                if changed {
                    crate::tools::record_undo(
                        &mut self.undo_stack,
                        &mut self.redo_stack,
                        UndoAction::Modify(idx, old),
                    );
                }
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
            clipboard_copy: None,
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
            self.screenshot.width as f32,
            self.canvas_height as f32,
            self.visible_buttons.len(),
        )
    }

    pub fn selection_hit(&self, pos: Point, preview: bool) -> Option<HitZone> {
        if preview {
            // The editor's UI bounds the workspace. Padding can start a mark,
            // but never resizes the underlying screenshot selection.
            (pos.x.is_finite() && pos.y.is_finite()).then_some(HitZone::Inside)
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
        if self.preview_mode {
            let (pixels, w, h, annotations) = self.export_selection()?;
            // With nothing to draw the crop is already the result; flattening
            // would copy it again, doubling peak memory (66 MB at 4K).
            if annotations.is_empty() {
                return Some((pixels, w, h));
            }
            return Some((
                crate::export::flatten_annotations(&pixels, w, h, &annotations),
                w,
                h,
            ));
        }
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

    /// Save uses the same canvas as Copy/Upload/Pin, with flattening on its worker.
    pub fn export_selection(&self) -> Option<(Vec<u8>, u32, u32, Vec<Annotation>)> {
        let r = self
            .selection
            .as_ref()?
            .clamped(self.screenshot.width, self.screenshot.height);
        let (pixels, w, h) = self.raw_selection()?;
        let annotations = self
            .annotations
            .iter()
            .map(|a| crate::export::offset_annotation(a, r.x as f32, r.y as f32))
            .collect();
        if self.preview_mode {
            crate::canvas::prepare_export(pixels, w, h, annotations)
        } else {
            Some((pixels, w, h, annotations))
        }
    }

    /// Export extents in capture coordinates, excluding unfinished annotations.
    pub fn export_canvas_bounds(&self) -> Option<crate::canvas::CanvasBounds> {
        let r = self
            .selection
            .as_ref()?
            .clamped(self.screenshot.width, self.screenshot.height);
        if r.w == 0 || r.h == 0 {
            return None;
        }
        let mut b = crate::canvas::CanvasBounds {
            x: r.x as f32,
            y: r.y as f32,
            width: r.w as f32,
            height: r.h as f32,
        };
        if self.preview_mode {
            for a in &self.annotations {
                b.include(a);
            }
        }
        Some(b)
    }

    pub fn export_screen_origin(&self) -> Option<(i32, i32)> {
        let b = self.export_canvas_bounds()?;
        Some((
            (b.x as i32).saturating_add(self.screenshot.x_offset),
            (b.y as i32).saturating_add(self.screenshot.y_offset),
        ))
    }

    pub fn export_failure_message(&self) -> &'static str {
        if self
            .export_canvas_bounds()
            .is_some_and(|b| b.export_size().is_none())
        {
            "Canvas is too large. Move or resize annotations closer to the image."
        } else {
            "Could not prepare the image. Your image is still here; try again."
        }
    }

    pub fn editor_canvas_bounds(&self) -> crate::canvas::CanvasBounds {
        let mut bounds = crate::canvas::bounds(
            self.screenshot.width,
            self.screenshot.height,
            &self.annotations,
        );
        if let Some(a) = self.in_progress() {
            bounds.include(&a);
        }
        if self.text_input_active && !self.text_input_buffer.trim().is_empty() {
            bounds.include(&Annotation::Text {
                position: self.text_input_position,
                text: self.text_input_buffer.clone(),
                color: self.current_color,
                font_size: self.text_input_font_size,
            });
        }
        bounds
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
