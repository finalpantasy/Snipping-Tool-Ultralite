use super::{Annotation, AnnotationTool};
use crate::geometry::{Color, Point};

pub struct CircleTool {
    color: Color,
    thickness: f32,
    start: Option<Point>,
    current: Option<Point>,
    drawing: bool,
}

impl CircleTool {
    pub fn new(color: Color, thickness: f32) -> Self {
        Self {
            color,
            thickness: thickness.clamp(1.0, 20.0),
            start: None,
            current: None,
            drawing: false,
        }
    }

    pub fn set_color(&mut self, color: Color) {
        self.color = color;
    }

    pub fn set_thickness(&mut self, thickness: f32) {
        self.thickness = thickness.clamp(1.0, 20.0);
    }

    fn make_annotation(&self, end: Point) -> Option<Annotation> {
        let start = self.start?;
        let min_x = start.x.min(end.x);
        let min_y = start.y.min(end.y);
        let max_x = start.x.max(end.x);
        let max_y = start.y.max(end.y);
        let center = Point::new((min_x + max_x) / 2.0, (min_y + max_y) / 2.0);
        let radius_x = (max_x - min_x) / 2.0;
        let radius_y = (max_y - min_y) / 2.0;
        if radius_x < 1.0 && radius_y < 1.0 {
            return None; // too small — likely a click, not a drag
        }
        Some(Annotation::Ellipse {
            center,
            radius_x,
            radius_y,
            color: self.color,
            thickness: self.thickness,
        })
    }
}

impl AnnotationTool for CircleTool {
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
