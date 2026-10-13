use super::{Annotation, AnnotationTool};
use crate::geometry::{Color, Point};

pub struct TextTool {
    pending_position: Option<Point>,
    color: Color,
    font_size: f32,
}

impl TextTool {
    pub fn new(color: Color, font_size: f32) -> Self {
        Self {
            pending_position: None,
            color,
            font_size: font_size.clamp(8.0, 128.0),
        }
    }

    pub fn set_color(&mut self, color: Color) {
        self.color = color;
    }

    pub fn set_font_size(&mut self, font_size: f32) {
        self.font_size = font_size.clamp(8.0, 128.0);
    }

    pub fn color(&self) -> Color {
        self.color
    }

    pub fn font_size(&self) -> f32 {
        self.font_size
    }

    /// Take the pending position, leaving `None` in its place.
    pub fn take_pending_position(&mut self) -> Option<Point> {
        self.pending_position.take()
    }

    pub fn pending_position(&self) -> Option<Point> {
        self.pending_position
    }
}

impl AnnotationTool for TextTool {
    fn on_mouse_down(&mut self, pos: Point) {
        self.pending_position = Some(pos);
    }

    fn on_mouse_move(&mut self, _pos: Point) {
        // No-op: text placement is a single click, not a drag.
    }

    fn on_mouse_up(&mut self, _pos: Point) -> Option<Annotation> {
        // Text annotations are committed via the text input flow,
        // not directly from mouse_up.
        None
    }

    fn is_drawing(&self) -> bool {
        false
    }

    fn in_progress_annotation(&self) -> Option<Annotation> {
        None
    }
}
