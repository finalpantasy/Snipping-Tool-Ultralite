use super::selection::Selection;
use crate::geometry::Point;
use crate::tools::ToolKind;

pub const TOOLBAR_HEIGHT: f32 = 40.0;
pub const TOOLBAR_PADDING: f32 = 8.0;
pub const BUTTON_SIZE: f32 = 32.0;
pub const BUTTON_COUNT: usize = 24;

/// What a toolbar button does when clicked.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ButtonAction {
    /// Switch the active annotation tool.
    Tool(ToolKind),
    /// Select a preset color (index into `Color::presets()`).
    Color(usize),
    Ocr,
    Upload,
    Pin,
    Copy,
    Save,
}

/// Static definition of one toolbar button.
pub struct ButtonDef {
    pub action: ButtonAction,
    /// Lucide icon name rendered by icons.rs; None for color swatches.
    pub icon: Option<&'static str>,
    pub tooltip: &'static str,
}

/// Single source of truth for the toolbar. Array order defines the "original"
/// button indices that `ToolbarConfig::visible_button_indices()` refers to:
/// 0-13 tools, 14-18 color swatches, 19-23 actions.
pub const BUTTONS: [ButtonDef; BUTTON_COUNT] = [
    ButtonDef {
        action: ButtonAction::Tool(ToolKind::Select),
        icon: Some("select"),
        tooltip: "Select annotations (V)",
    },
    ButtonDef {
        action: ButtonAction::Tool(ToolKind::Arrow),
        icon: Some("arrow"),
        tooltip: "Arrow (A)",
    },
    ButtonDef {
        action: ButtonAction::Tool(ToolKind::Rectangle),
        icon: Some("rectangle"),
        tooltip: "Rectangle (R)",
    },
    ButtonDef {
        action: ButtonAction::Tool(ToolKind::Circle),
        icon: Some("circle"),
        tooltip: "Circle (C)",
    },
    ButtonDef {
        action: ButtonAction::Tool(ToolKind::RoundedRect),
        icon: Some("rounded-rect"),
        tooltip: "Rounded Rect (O)",
    },
    ButtonDef {
        action: ButtonAction::Tool(ToolKind::Line),
        icon: Some("line"),
        tooltip: "Line (L)",
    },
    ButtonDef {
        action: ButtonAction::Tool(ToolKind::Pencil),
        icon: Some("pencil"),
        tooltip: "Pencil (P)",
    },
    ButtonDef {
        action: ButtonAction::Tool(ToolKind::Highlight),
        icon: Some("highlight"),
        tooltip: "Highlight (H)",
    },
    ButtonDef {
        action: ButtonAction::Tool(ToolKind::Spotlight),
        icon: Some("spotlight"),
        tooltip: "Spotlight (F)",
    },
    ButtonDef {
        action: ButtonAction::Tool(ToolKind::Text),
        icon: Some("text"),
        tooltip: "Text (T)",
    },
    ButtonDef {
        action: ButtonAction::Tool(ToolKind::Pixelate),
        icon: Some("pixelate"),
        tooltip: "Pixelate (B)",
    },
    ButtonDef {
        action: ButtonAction::Tool(ToolKind::StepMarker),
        icon: Some("step-marker"),
        tooltip: "Step Marker (N)",
    },
    ButtonDef {
        action: ButtonAction::Tool(ToolKind::Eyedropper),
        icon: Some("eyedropper"),
        tooltip: "Eyedropper (I)",
    },
    ButtonDef {
        action: ButtonAction::Tool(ToolKind::Measurement),
        icon: Some("measurement"),
        tooltip: "Measurement (M)",
    },
    ButtonDef {
        action: ButtonAction::Color(0),
        icon: None,
        tooltip: "Red #ff0000 (right-click: pick)",
    },
    ButtonDef {
        action: ButtonAction::Color(1),
        icon: None,
        tooltip: "Blue #0066ff (right-click: pick)",
    },
    ButtonDef {
        action: ButtonAction::Color(2),
        icon: None,
        tooltip: "Green #00c000 (right-click: pick)",
    },
    ButtonDef {
        action: ButtonAction::Color(3),
        icon: None,
        tooltip: "Yellow #ffdd00 (right-click: pick)",
    },
    ButtonDef {
        action: ButtonAction::Color(4),
        icon: None,
        tooltip: "Purple #a000ff (right-click: pick)",
    },
    ButtonDef {
        action: ButtonAction::Ocr,
        icon: Some("ocr"),
        tooltip: "OCR (Extract Text)",
    },
    ButtonDef {
        action: ButtonAction::Upload,
        icon: Some("upload"),
        tooltip: "Upload (Imgur)",
    },
    ButtonDef {
        action: ButtonAction::Pin,
        icon: Some("pin"),
        tooltip: "Pin",
    },
    ButtonDef {
        action: ButtonAction::Copy,
        icon: Some("copy"),
        tooltip: "Copy (Ctrl+C)",
    },
    ButtonDef {
        action: ButtonAction::Save,
        icon: Some("save"),
        tooltip: "Save (Ctrl+S)",
    },
];

/// Toolbar that appears near the selection rectangle. Button layout/order is
/// defined by [`BUTTONS`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Toolbar {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Toolbar {
    /// Total width of the toolbar based on button count and padding.
    pub fn toolbar_width() -> f32 {
        TOOLBAR_PADDING + (BUTTON_COUNT as f32) * (BUTTON_SIZE + TOOLBAR_PADDING)
    }

    /// Total width for a dynamic number of visible buttons.
    pub fn toolbar_width_dynamic(visible_count: usize) -> f32 {
        TOOLBAR_PADDING + (visible_count as f32) * (BUTTON_SIZE + TOOLBAR_PADDING)
    }

    /// Position the toolbar centered below (or above) the selection.
    pub fn position_for(selection: &Selection, screen_height: f32) -> Self {
        Self::position_for_dynamic(selection, screen_height, BUTTON_COUNT)
    }

    /// Position the toolbar with a dynamic button count.
    pub fn position_for_dynamic(
        selection: &Selection,
        screen_height: f32,
        visible_count: usize,
    ) -> Self {
        let width = Self::toolbar_width_dynamic(visible_count);
        let height = TOOLBAR_HEIGHT;

        // Center horizontally on the selection, clamped to screen bounds
        let x = (selection.x + (selection.width - width) / 2.0).max(0.0);

        // Place below the selection by default
        let below_y = selection.y + selection.height + TOOLBAR_PADDING;

        // Flip above if toolbar would go off screen bottom, clamp to 0 minimum
        let y = if below_y + height > screen_height {
            (selection.y - height - TOOLBAR_PADDING).max(0.0)
        } else {
            below_y
        };

        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Hit-test a point, returning the button index if hit.
    pub fn hit_test(&self, point: Point) -> Option<usize> {
        self.hit_test_dynamic(point, BUTTON_COUNT)
    }

    /// Hit-test with a dynamic visible button count, returning the visible index.
    pub fn hit_test_dynamic(&self, point: Point, visible_count: usize) -> Option<usize> {
        // Check if point is within toolbar bounds first
        if point.x < self.x
            || point.x > self.x + self.width
            || point.y < self.y
            || point.y > self.y + self.height
        {
            return None;
        }

        for i in 0..visible_count {
            let (bx, by, bw, bh) = self.button_rect(i);
            if point.x >= bx && point.x <= bx + bw && point.y >= by && point.y <= by + bh {
                return Some(i);
            }
        }
        None
    }

    /// Get the rectangle (x, y, width, height) for a button by index.
    pub fn button_rect(&self, index: usize) -> (f32, f32, f32, f32) {
        let bx = self.x + TOOLBAR_PADDING + (index as f32) * (BUTTON_SIZE + TOOLBAR_PADDING);
        let by = self.y + (self.height - BUTTON_SIZE) / 2.0;
        (bx, by, BUTTON_SIZE, BUTTON_SIZE)
    }
}
