use super::{Annotation, AnnotationTool};
use crate::geometry::{Color, Point, Size};

pub struct RoundedRectTool {
    color: Color,
    thickness: f32,
    radius: f32,
    start: Option<Point>,
    current: Option<Point>,
    drawing: bool,
}

impl RoundedRectTool {
    pub fn new(color: Color, thickness: f32) -> Self {
        Self {
            color,
            thickness: thickness.clamp(1.0, 20.0),
            radius: 10.0,
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

    pub fn radius(&self) -> f32 {
        self.radius
    }

    pub fn set_radius(&mut self, r: f32) {
        self.radius = r.clamp(2.0, 50.0);
    }

    fn make_annotation(&self, end: Point) -> Option<Annotation> {
        let start = self.start?;
        let min_x = start.x.min(end.x);
        let min_y = start.y.min(end.y);
        let max_x = start.x.max(end.x);
        let max_y = start.y.max(end.y);
        let w = max_x - min_x;
        let h = max_y - min_y;
        if w < 2.0 || h < 2.0 {
            return None;
        }
        Some(Annotation::RoundedRect {
            top_left: Point::new(min_x, min_y),
            size: Size::new(w, h),
            color: self.color,
            thickness: self.thickness,
            radius: self.radius,
        })
    }
}

impl AnnotationTool for RoundedRectTool {
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
