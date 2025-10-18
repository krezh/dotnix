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
    /// Point mode - select a single point instead of region
    point_mode: bool,
    /// Aspect ratio constraint (if any)
    aspect_ratio: Option<f64>,
    /// Snap target window rectangle (when hovering over a window)
    snap_target: Option<Rect>,
    /// Animated snap target rectangle (for smooth transitions)
    animated_snap_target: Option<Rect>,
}

impl Selection {
    pub fn new(point_mode: bool, aspect_ratio: Option<f64>) -> Self {
        Self {
            mode: SelectionMode::Hover,
            hover_pos: (0, 0),
            rect: None,
            point_mode,
            aspect_ratio,
            snap_target: None,
            animated_snap_target: None,
        }
    }

    pub fn start_selection(&mut self, x: i32, y: i32) {
        self.mode = SelectionMode::Selecting;
        self.hover_pos = (x, y);

        if self.point_mode {
            // In point mode, immediately create a 1x1 selection
            self.rect = Some(Rect::new(x, y, 1, 1));
            self.mode = SelectionMode::Complete;
        } else {
            self.rect = Some(Rect::new(x, y, 0, 0));
        }
    }

    pub fn update_drag(&mut self, start_x: i32, start_y: i32, current_x: i32, current_y: i32) {
        if self.mode == SelectionMode::Selecting {
            let mut rect = Rect::from_points(start_x, start_y, current_x, current_y);

            // Apply aspect ratio constraint if specified
            if let Some(ratio) = self.aspect_ratio {
                let current_ratio = rect.width as f64 / rect.height as f64;

                if current_ratio > ratio {
                    // Width is too large, adjust it
                    rect.width = (rect.height as f64 * ratio) as i32;
                } else if current_ratio < ratio {
                    // Height is too large, adjust it
                    rect.height = (rect.width as f64 / ratio) as i32;
                }
            }

            self.rect = Some(rect);
        }
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
    fn test_selection_point_mode() {
        let mut sel = Selection::new(true, None);
        sel.start_selection(100, 200);

        let rect = sel.get_selection().expect("Selection should be valid");
        assert_eq!(rect.x, 100);
        assert_eq!(rect.y, 200);
        assert_eq!(rect.width, 1);
        assert_eq!(rect.height, 1);
    }

    #[test]
    fn test_selection_region_mode() {
        let mut sel = Selection::new(false, None);
        sel.start_selection(10, 10);

        sel.update_drag(10, 10, 50, 40);

        let rect = sel.get_selection().expect("Selection should be valid");
        assert_eq!(rect.width, 40);
        assert_eq!(rect.height, 30);
    }
}
