use super::{Annotation, AnnotationTool};
use crate::geometry::{Point, Size};

pub struct PixelateTool {
    start: Option<Point>,
    current: Option<Point>,
    block_size: u8,
    drawing: bool,
}

impl PixelateTool {
    pub fn block_size(&self) -> u8 {
        self.block_size
    }
    pub fn set_block_size(&mut self, size: u8) {
        self.block_size = size.clamp(2, 64);
    }
    pub fn new(block_size: u8) -> Self {
        Self {
            start: None,
            current: None,
            block_size: if block_size == 0 { 10 } else { block_size },
            drawing: false,
        }
    }

    fn make_annotation(&self, end: Point) -> Option<Annotation> {
        let start = self.start?;
        let min_x = start.x.min(end.x);
        let min_y = start.y.min(end.y);
        let max_x = start.x.max(end.x);
        let max_y = start.y.max(end.y);
        let w = max_x - min_x;
        let h = max_y - min_y;
        if w < 1.0 || h < 1.0 {
            return None;
        }
        Some(Annotation::Pixelate {
            top_left: Point::new(min_x, min_y),
            size: Size::new(w, h),
            block_size: self.block_size,
        })
    }
}

impl AnnotationTool for PixelateTool {
    fn on_mouse_down(&mut self, pos: Point) {
        self.start = Some(pos);
        self.current = Some(pos);
        self.drawing = true;
    }

    fn on_mouse_move(&mut self, pos: Point) {
        if self.drawing {
            self.current = Some(pos);
        }
    }

    fn on_mouse_up(&mut self, pos: Point) -> Option<Annotation> {
        if !self.drawing {
            return None;
        }
        self.drawing = false;
        let annotation = self.make_annotation(pos);
        self.start = None;
        self.current = None;
        annotation
    }

    fn is_drawing(&self) -> bool {
        self.drawing
    }

    fn in_progress_annotation(&self) -> Option<Annotation> {
        if !self.drawing {
            return None;
        }
        self.current.and_then(|c| self.make_annotation(c))
    }
}
