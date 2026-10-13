//! Contextual size editing and atomic annotation deletion.
use crate::{
    state::OverlayState,
    tools::{self, Annotation, ToolKind, UndoAction},
};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SizeKind {
    Width,
    Text,
    Step,
    Pixelate,
    None,
}
impl SizeKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Width => "Width",
            Self::Text => "Size",
            Self::Step => "Diameter",
            Self::Pixelate => "Blocks",
            Self::None => "Drag to size",
        }
    }
    pub fn limits(self) -> (f32, f32) {
        match self {
            Self::Width => (1., 20.),
            Self::Text => (8., 128.),
            Self::Step => (16., 60.),
            Self::Pixelate => (2., 64.),
            Self::None => (0., 0.),
        }
    }
}
fn kind(a: &Annotation) -> SizeKind {
    match a {
        Annotation::Text { .. } => SizeKind::Text,
        Annotation::StepMarker { .. } => SizeKind::Step,
        Annotation::Pixelate { .. } => SizeKind::Pixelate,
        Annotation::Arrow { .. }
        | Annotation::Pencil { .. }
        | Annotation::Rectangle { .. }
        | Annotation::RoundedRect { .. }
        | Annotation::Ellipse { .. }
        | Annotation::Line { .. } => SizeKind::Width,
        _ => SizeKind::None,
    }
}
impl OverlayState {
    pub fn size_kind(&self) -> SizeKind {
        self.selected_index
            .and_then(|i| self.annotations.get(i))
            .map(kind)
            .unwrap_or(match self.active_tool {
                ToolKind::Text => SizeKind::Text,
                ToolKind::StepMarker => SizeKind::Step,
                ToolKind::Pixelate => SizeKind::Pixelate,
                ToolKind::Select
                | ToolKind::Highlight
                | ToolKind::Spotlight
                | ToolKind::Measurement
                | ToolKind::Eyedropper => SizeKind::None,
                _ => SizeKind::Width,
            })
    }
    pub fn tool_size(&self) -> f32 {
        if let Some(a) = self.selected_index.and_then(|i| self.annotations.get(i)) {
            return match a {
                Annotation::Text { font_size, .. } => *font_size,
                Annotation::StepMarker { size, .. } => *size,
                Annotation::Pixelate { block_size, .. } => *block_size as f32,
                Annotation::Arrow { thickness, .. }
                | Annotation::Pencil { thickness, .. }
                | Annotation::Rectangle { thickness, .. }
                | Annotation::RoundedRect { thickness, .. }
                | Annotation::Ellipse { thickness, .. }
                | Annotation::Line { thickness, .. } => *thickness,
                _ => 0.,
            };
        }
        match self.size_kind() {
            SizeKind::Text => {
                if self.text_input_active {
                    self.text_input_font_size
                } else {
                    self.text_tool.font_size()
                }
            }
            SizeKind::Step => self.step_marker_tool.size(),
            SizeKind::Pixelate => self.pixelate_tool.block_size() as f32,
            SizeKind::Width => self.current_thickness,
            SizeKind::None => 0.,
        }
    }
    pub fn set_tool_size(&mut self, value: f32, record: bool) {
        if !value.is_finite() || self.size_kind() == SizeKind::None {
            return;
        }
        let (lo, hi) = self.size_kind().limits();
        let value = value.clamp(lo, hi);
        if let Some(i) = self.selected_index {
            let old = self.annotations[i].clone();
            match &mut self.annotations[i] {
                Annotation::Text { font_size, .. } => *font_size = value,
                Annotation::StepMarker { size, .. } => *size = value,
                Annotation::Pixelate { block_size, .. } => *block_size = value.round() as u8,
                Annotation::Arrow { thickness, .. }
                | Annotation::Pencil { thickness, .. }
                | Annotation::Rectangle { thickness, .. }
                | Annotation::RoundedRect { thickness, .. }
                | Annotation::Ellipse { thickness, .. }
                | Annotation::Line { thickness, .. } => *thickness = value,
                _ => {}
            }
            if record && self.annotations[i] != old {
                tools::record_undo(
                    &mut self.undo_stack,
                    &mut self.redo_stack,
                    UndoAction::Modify(i, old),
                );
            }
        } else {
            match self.size_kind() {
                SizeKind::Width => self.set_thickness_all(value),
                SizeKind::Text => {
                    self.text_tool.set_font_size(value);
                    self.text_input_font_size = value;
                }
                SizeKind::Step => self.step_marker_tool.set_size(value),
                SizeKind::Pixelate => self.pixelate_tool.set_block_size(value.round() as u8),
                SizeKind::None => {}
            }
        }
    }
    pub fn delete_annotation(&mut self, i: usize) -> bool {
        if i >= self.annotations.len() {
            return false;
        }
        let removed = self.annotations.remove(i);
        let mut undo = Vec::new();
        let mut number = 1;
        for (j, a) in self.annotations.iter_mut().enumerate() {
            if let Annotation::StepMarker { number: n, .. } = a {
                if *n != number {
                    let old = a.clone();
                    if let Annotation::StepMarker { number: n, .. } = a {
                        *n = number;
                    }
                    undo.push(UndoAction::Modify(j, old));
                }
                number += 1;
            }
        }
        undo.push(UndoAction::Delete(i, removed));
        tools::record_undo(
            &mut self.undo_stack,
            &mut self.redo_stack,
            UndoAction::Batch(undo),
        );
        self.selected_index = None;
        self.sync_step_counter();
        true
    }
    pub fn in_progress(&self) -> Option<Annotation> {
        use tools::AnnotationTool;
        match self.active_tool {
            ToolKind::Arrow => self.arrow_tool.in_progress_annotation(),
            ToolKind::Rectangle => self.rectangle_tool.in_progress_annotation(),
            ToolKind::RoundedRect => self.rounded_rect_tool.in_progress_annotation(),
            ToolKind::Circle => self.circle_tool.in_progress_annotation(),
            ToolKind::Line => self.line_tool.in_progress_annotation(),
            ToolKind::Pencil => self.pencil_tool.in_progress_annotation(),
            ToolKind::Highlight => self.highlight_tool.in_progress_annotation(),
            ToolKind::Pixelate => self.pixelate_tool.in_progress_annotation(),
            ToolKind::Spotlight => self.spotlight_tool.in_progress_annotation(),
            ToolKind::Measurement => self.measurement_tool.in_progress_annotation(),
            _ => None,
        }
    }
}
