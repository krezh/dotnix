//! Shapes in a square, as data: what something with its own drawing paints, for any frontend.

/// Red, green, blue and alpha, each from 0 to 1.
pub type Color = [f32; 4];

pub enum Shape {
    Fill {
        points: Vec<(f32, f32)>,
        color: Color,
    },
    Stroke {
        points: Vec<(f32, f32)>,
        width: f32,
        color: Color,
    },
    /// Whole pixels stay whole, unlike a filled outline's edge.
    Circle {
        left: f32,
        top: f32,
        size: f32,
        color: u32,
    },
}

/// Points are from the square's top left.
pub struct Drawing {
    pub side: f32,
    pub shapes: Vec<Shape>,
}
