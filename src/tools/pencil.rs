use super::{Annotation, AnnotationTool};
use crate::geometry::{Color, Point};

/// Minimum squared distance between consecutive points to reduce memory usage.
const MIN_POINT_DISTANCE_SQ: f32 = 0.25; // preserve half-pixel detail

pub struct PencilTool {
    color: Color,
    thickness: f32,
    points: Vec<Point>,
    drawing: bool,
}

impl PencilTool {
    pub fn new(color: Color, thickness: f32) -> Self {
        Self {
            color,
            thickness: thickness.clamp(1.0, 20.0),
            points: Vec::new(),
            drawing: false,
        }
    }

    pub fn set_color(&mut self, color: Color) {
        self.color = color;
    }

    pub fn set_thickness(&mut self, thickness: f32) {
        self.thickness = thickness.clamp(1.0, 20.0);
    }
}

impl AnnotationTool for PencilTool {
    fn on_mouse_down(&mut self, pos: Point) {
        self.points.clear();
        self.points.push(pos);
        self.drawing = true;
    }

    fn on_mouse_move(&mut self, pos: Point) {
        if self.drawing {
            if let Some(last) = self.points.last() {
                let dx = pos.x - last.x;
                let dy = pos.y - last.y;
                if dx * dx + dy * dy < MIN_POINT_DISTANCE_SQ {
                    return;
                }
            }
            self.points.push(pos);
        }
    }

    fn on_mouse_up(&mut self, pos: Point) -> Option<Annotation> {
        if !self.drawing {
            return None;
        }
        if self.points.last().map_or(true, |p| {
            (p.x - pos.x).powi(2) + (p.y - pos.y).powi(2) > 0.0001
        }) {
            self.points.push(pos);
        }
        self.drawing = false;
        let annotation = Annotation::Pencil {
            points: std::mem::take(&mut self.points),
            color: self.color,
            thickness: self.thickness,
        };
        Some(annotation)
    }

    fn is_drawing(&self) -> bool {
        self.drawing
    }

    fn in_progress_annotation(&self) -> Option<Annotation> {
        if !self.drawing {
            return None;
        }
        Some(Annotation::Pencil {
            points: self.points.clone(),
            color: self.color,
            thickness: self.thickness,
        })
    }
}
