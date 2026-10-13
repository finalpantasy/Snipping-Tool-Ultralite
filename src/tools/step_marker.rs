use super::{Annotation, AnnotationTool};
use crate::geometry::{Color, Point};

pub struct StepMarkerTool {
    color: Color,
    size: f32,
    next_number: u32,
    click_pos: Option<Point>,
}

impl StepMarkerTool {
    pub fn new(color: Color, size: f32) -> Self {
        Self {
            color,
            size: size.clamp(16.0, 60.0),
            next_number: 1,
            click_pos: None,
        }
    }

    pub fn set_color(&mut self, color: Color) {
        self.color = color;
    }

    pub fn set_size(&mut self, size: f32) {
        self.size = size.clamp(16.0, 60.0);
    }

    pub fn size(&self) -> f32 {
        self.size
    }

    pub fn reset_counter(&mut self) {
        self.next_number = 1;
    }

    /// Set the next number directly — used to resync after undo/redo/delete.
    pub fn set_next_number(&mut self, n: u32) {
        self.next_number = n.max(1);
    }
}

impl AnnotationTool for StepMarkerTool {
    fn on_mouse_down(&mut self, pos: Point) {
        self.click_pos = Some(pos);
    }

    fn on_mouse_move(&mut self, _pos: Point) {
        // No drag behavior
    }

    fn on_mouse_up(&mut self, pos: Point) -> Option<Annotation> {
        self.click_pos.take()?;
        let ann = Annotation::StepMarker {
            position: pos,
            number: self.next_number,
            color: self.color,
            size: self.size,
        };
        self.next_number = self.next_number.saturating_add(1);
        Some(ann)
    }

    fn is_drawing(&self) -> bool {
        false
    }

    fn in_progress_annotation(&self) -> Option<Annotation> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn step_marker_produces_correct_number() {
        let mut tool = StepMarkerTool::new(Color::red(), 28.0);
        tool.on_mouse_down(Point::new(100.0, 200.0));
        let ann = tool.on_mouse_up(Point::new(100.0, 200.0));
        assert!(ann.is_some());
        if let Some(Annotation::StepMarker { number, .. }) = ann {
            assert_eq!(number, 1);
        } else {
            panic!("Expected StepMarker annotation");
        }
    }

    #[test]
    fn counter_auto_increments() {
        let mut tool = StepMarkerTool::new(Color::red(), 28.0);

        tool.on_mouse_down(Point::new(10.0, 10.0));
        let a1 = tool.on_mouse_up(Point::new(10.0, 10.0));

        tool.on_mouse_down(Point::new(50.0, 50.0));
        let a2 = tool.on_mouse_up(Point::new(50.0, 50.0));

        match (a1, a2) {
            (
                Some(Annotation::StepMarker { number: n1, .. }),
                Some(Annotation::StepMarker { number: n2, .. }),
            ) => {
                assert_eq!(n1, 1);
                assert_eq!(n2, 2);
            }
            _ => panic!("Expected two StepMarker annotations"),
        }
    }

    #[test]
    fn render_smoke_test() {
        use crate::tools::render_annotation;
        use tiny_skia::Pixmap;

        let ann = Annotation::StepMarker {
            position: Point::new(50.0, 50.0),
            number: 1,
            color: Color::red(),
            size: 28.0,
        };
        let mut pixmap = Pixmap::new(100, 100).unwrap();
        // Should not panic
        render_annotation(&ann, &mut pixmap, None, None);
    }
}
