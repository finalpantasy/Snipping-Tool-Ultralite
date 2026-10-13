#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

impl Size {
    pub fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self {
            r: r.clamp(0.0, 1.0),
            g: g.clamp(0.0, 1.0),
            b: b.clamp(0.0, 1.0),
            a: a.clamp(0.0, 1.0),
        }
    }

    const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self {
            r: r as f32 / 255.,
            g: g as f32 / 255.,
            b: b as f32 / 255.,
            a: 1.,
        }
    }

    // Saturated annotation colors, independent of the interface theme.
    pub const fn red() -> Self {
        Self::rgb(255, 0, 0) // #ff0000
    }

    pub const fn blue() -> Self {
        Self::rgb(0, 102, 255) // #0066ff
    }

    pub const fn green() -> Self {
        Self::rgb(0, 192, 0) // #00c000
    }

    pub const fn yellow() -> Self {
        Self::rgb(255, 221, 0) // #ffdd00
    }

    pub const fn white() -> Self {
        Self::rgb(255, 255, 255)
    }

    pub const fn mauve() -> Self {
        Self::rgb(160, 0, 255) // #a000ff, purple; legacy name kept for saved settings
    }

    pub const fn peach() -> Self {
        Self::rgb(255, 109, 0) // #ff6d00, orange
    }

    pub const fn teal() -> Self {
        Self::rgb(0, 191, 165) // #00bfa5
    }

    pub const fn sky() -> Self {
        Self::rgb(0, 184, 255) // #00b8ff
    }

    pub const fn lavender() -> Self {
        Self::rgb(124, 77, 255) // #7c4dff
    }

    pub fn presets() -> &'static [Self] {
        static PRESETS: &[Color] = &[
            Color::red(),
            Color::blue(),
            Color::green(),
            Color::yellow(),
            Color::mauve(),
        ];
        PRESETS
    }
}

impl From<Color> for tiny_skia::Color {
    fn from(c: Color) -> Self {
        tiny_skia::Color::from_rgba(c.r, c.g, c.b, c.a).unwrap_or(tiny_skia::Color::BLACK)
    }
}
