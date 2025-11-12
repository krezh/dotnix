/// Rectangular region in screen coordinates
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Rect {
    /// Creates a new rectangle
    #[inline]
    pub const fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Check if this rectangle intersects with another rectangle
    #[inline]
    pub const fn intersects(&self, other: &Rect) -> bool {
        self.x < other.x + other.width
            && self.x + self.width > other.x
            && self.y < other.y + other.height
            && self.y + self.height > other.y
    }

    /// Creates a rectangle from two corner points
    ///
    /// Automatically normalizes so top-left is at (min_x, min_y)
    #[inline]
    pub fn from_points(x1: i32, y1: i32, x2: i32, y2: i32) -> Self {
        let x = x1.min(x2);
        let y = y1.min(y2);
        let width = (x1 - x2).abs();
        let height = (y1 - y2).abs();

        log::debug!(
            "Rect::from_points({},{},{},{}) -> x={} y={} w={} h={}",
            x1,
            y1,
            x2,
            y2,
            x,
            y,
            width,
            height
        );

        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Checks if this rectangle has non-zero area
    #[inline]
    pub const fn is_valid(&self) -> bool {
        self.width > 0 && self.height > 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SelectionMode {
    /// Hovering, not selecting
    Hover,
    /// Actively dragging to create a selection
    Selecting,
    /// Selection complete
    Complete,
}

#[derive(Clone)]
pub struct Selection {
    mode: SelectionMode,
    /// Current hover position (for highlighting)
    hover_pos: (i32, i32),
    /// Selection rectangle (if any)
    rect: Option<Rect>,
    /// Snap target window rectangle (when hovering over a window)
    snap_target: Option<Rect>,
    /// Animated snap target rectangle (for smooth transitions)
    animated_snap_target: Option<Rect>,
}

impl Selection {
    pub fn new() -> Self {
        Self {
            mode: SelectionMode::Hover,
            hover_pos: (0, 0),
            rect: None,
            snap_target: None,
            animated_snap_target: None,
        }
    }

    pub fn start_selection(&mut self, x: i32, y: i32) {
        self.mode = SelectionMode::Selecting;
        self.hover_pos = (x, y);
        self.rect = Some(Rect::new(x, y, 0, 0));
    }

    pub fn update_drag(&mut self, start_x: i32, start_y: i32, current_x: i32, current_y: i32) {
        if self.mode == SelectionMode::Selecting {
            let rect = Rect::from_points(start_x, start_y, current_x, current_y);
            self.rect = Some(rect);
        }
    }

    /// Create a selection from a rect
    pub fn from_rect(rect: Rect) -> Self {
        let mut selection = Self::new();
        selection.start_selection(rect.x, rect.y);
        selection.update_drag(rect.x, rect.y, rect.x + rect.width, rect.y + rect.height);
        selection
    }

    /// Returns the selection rectangle if it has non-zero area
    #[inline]
    pub fn get_selection(&self) -> Option<Rect> {
        self.rect.filter(|r| r.is_valid())
    }

    /// Returns the current rectangle (may be zero-sized during dragging)
    #[inline]
    pub const fn get_rect(&self) -> Option<Rect> {
        self.rect
    }

    pub fn set_snap_target(&mut self, target: Option<Rect>) {
        self.snap_target = target;
    }

    pub fn get_snap_target(&self) -> Option<Rect> {
        self.snap_target
    }

    pub fn set_animated_snap_target(&mut self, target: Option<Rect>) {
        self.animated_snap_target = target;
    }

    pub fn get_animated_snap_target(&self) -> Option<Rect> {
        self.animated_snap_target
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rect_from_points() {
        let rect = Rect::from_points(10, 10, 50, 40);
        assert_eq!(rect.x, 10);
        assert_eq!(rect.y, 10);
        assert_eq!(rect.width, 40);
        assert_eq!(rect.height, 30);

        // Test with reversed points
        let rect = Rect::from_points(50, 40, 10, 10);
        assert_eq!(rect.x, 10);
        assert_eq!(rect.y, 10);
        assert_eq!(rect.width, 40);
        assert_eq!(rect.height, 30);
    }

    #[test]
    fn test_selection_single_point() {
        let mut sel = Selection::new();
        sel.start_selection(100, 200);
        sel.update_drag(100, 200, 101, 201);

        let rect = sel.get_selection().expect("Selection should be valid");
        assert_eq!(rect.x, 100);
        assert_eq!(rect.y, 200);
        assert_eq!(rect.width, 1);
        assert_eq!(rect.height, 1);
    }

    #[test]
    fn test_selection_region() {
        let mut sel = Selection::new();
        sel.start_selection(10, 10);

        sel.update_drag(10, 10, 50, 40);

        let rect = sel.get_selection().expect("Selection should be valid");
        assert_eq!(rect.width, 40);
        assert_eq!(rect.height, 30);
    }
}
