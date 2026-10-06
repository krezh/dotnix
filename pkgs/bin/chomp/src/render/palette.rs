const MAX_WIDTH: f64 = 760.0;
const OUTER_MARGIN: f64 = 24.0;
const NARROW_MARGIN: f64 = 12.0;
const PADDING: f64 = 16.0;
const HEADING_HEIGHT: f64 = 16.0;
const HEADING_GAP: f64 = 8.0;
const ITEM_GAP: f64 = 10.0;
const SECTION_GAP: f64 = 16.0;
const MIN_ITEM_WIDTH: f64 = 132.0;
const MAX_ITEMS: usize = 7;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaletteAction {
    ScreenshotArea,
    ScreenshotScreen,
    ScreenshotWindow,
    Ocr,
    RecordArea,
    RecordScreen,
    RecordWindow,
    StopRecording,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PaletteRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl PaletteRect {
    pub fn contains(self, x: f64, y: f64) -> bool {
        x >= self.x && y >= self.y && x < self.x + self.width && y < self.y + self.height
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PaletteItem {
    pub action: PaletteAction,
    pub rect: PaletteRect,
}

const EMPTY_ITEM: PaletteItem = PaletteItem {
    action: PaletteAction::ScreenshotArea,
    rect: PaletteRect {
        x: 0.0,
        y: 0.0,
        width: 0.0,
        height: 0.0,
    },
};

pub struct ModePaletteLayout {
    pub bounds: PaletteRect,
    pub capture_heading_y: f64,
    pub record_heading_y: f64,
    items: [PaletteItem; MAX_ITEMS],
    item_count: usize,
}

impl ModePaletteLayout {
    pub fn new(
        width: i32,
        height: i32,
        configured_height: u32,
        is_recording: bool,
        supports_window_capture: bool,
        intro_progress: f64,
    ) -> Self {
        let screen_width = f64::from(width.max(1));
        let screen_height = f64::from(height.max(1));
        let margin = if screen_width < 480.0 {
            NARROW_MARGIN
        } else {
            OUTER_MARGIN
        };
        let panel_width = (screen_width - margin * 2.0).min(MAX_WIDTH).max(1.0);
        let content_width = (panel_width - PADDING * 2.0).max(1.0);
        let item_height = (f64::from(configured_height) * 0.75).clamp(38.0, 48.0);
        let columns = ((content_width + ITEM_GAP) / (MIN_ITEM_WIDTH + ITEM_GAP))
            .floor()
            .clamp(1.0, 4.0) as usize;

        let capture_count: usize = if supports_window_capture { 4 } else { 3 };
        let record_count: usize = if is_recording {
            1
        } else if supports_window_capture {
            3
        } else {
            2
        };
        let capture_rows = capture_count.div_ceil(columns);
        let record_columns = if is_recording { 1 } else { columns.min(3) };
        let record_rows = record_count.div_ceil(record_columns);
        let panel_height = PADDING * 2.0
            + HEADING_HEIGHT * 2.0
            + HEADING_GAP * 2.0
            + item_height * (capture_rows + record_rows) as f64
            + ITEM_GAP * (capture_rows.saturating_sub(1) + record_rows.saturating_sub(1)) as f64
            + SECTION_GAP;
        let intro = intro_progress.clamp(0.0, 1.0);
        let x = (screen_width - panel_width) / 2.0;
        let resting_y = (screen_height - panel_height - margin).max(margin);
        let y = resting_y + (panel_height + margin) * (1.0 - intro);

        let capture_heading_y = y + PADDING;
        let capture_y = capture_heading_y + HEADING_HEIGHT + HEADING_GAP;
        let record_heading_y = capture_y
            + item_height * capture_rows as f64
            + ITEM_GAP * capture_rows.saturating_sub(1) as f64
            + SECTION_GAP;
        let record_y = record_heading_y + HEADING_HEIGHT + HEADING_GAP;

        let mut layout = Self {
            bounds: PaletteRect {
                x,
                y,
                width: panel_width,
                height: panel_height,
            },
            capture_heading_y,
            record_heading_y,
            items: [EMPTY_ITEM; MAX_ITEMS],
            item_count: 0,
        };

        let capture_actions = [
            PaletteAction::ScreenshotArea,
            PaletteAction::ScreenshotScreen,
            PaletteAction::ScreenshotWindow,
            PaletteAction::Ocr,
        ];
        for (index, action) in capture_actions
            .into_iter()
            .filter(|action| supports_window_capture || *action != PaletteAction::ScreenshotWindow)
            .enumerate()
        {
            layout.push_grid_item(
                action,
                index,
                columns,
                capture_count,
                capture_y,
                content_width,
                item_height,
            );
        }

        if is_recording {
            layout.push_grid_item(
                PaletteAction::StopRecording,
                0,
                1,
                1,
                record_y,
                content_width,
                item_height,
            );
        } else {
            let record_actions = [
                PaletteAction::RecordArea,
                PaletteAction::RecordScreen,
                PaletteAction::RecordWindow,
            ];
            for (index, action) in record_actions
                .into_iter()
                .filter(|action| supports_window_capture || *action != PaletteAction::RecordWindow)
                .enumerate()
            {
                layout.push_grid_item(
                    action,
                    index,
                    record_columns,
                    record_count,
                    record_y,
                    content_width,
                    item_height,
                );
            }
        }

        layout
    }

    pub fn items(&self) -> &[PaletteItem] {
        &self.items[..self.item_count]
    }

    pub fn action_at(&self, x: f64, y: f64) -> Option<PaletteAction> {
        self.items()
            .iter()
            .find(|item| item.rect.contains(x, y))
            .map(|item| item.action)
    }

    fn push_grid_item(
        &mut self,
        action: PaletteAction,
        index: usize,
        columns: usize,
        item_count: usize,
        y: f64,
        content_width: f64,
        item_height: f64,
    ) {
        let row = index / columns;
        let column = index % columns;
        let items_in_row = (item_count - row * columns).min(columns);
        let item_width = (content_width - ITEM_GAP * items_in_row.saturating_sub(1) as f64)
            / items_in_row as f64;
        let row_width =
            item_width * items_in_row as f64 + ITEM_GAP * items_in_row.saturating_sub(1) as f64;
        let row_x = self.bounds.x + PADDING + (content_width - row_width) / 2.0;

        self.items[self.item_count] = PaletteItem {
            action,
            rect: PaletteRect {
                x: row_x + column as f64 * (item_width + ITEM_GAP),
                y: y + row as f64 * (item_height + ITEM_GAP),
                width: item_width,
                height: item_height,
            },
        };
        self.item_count += 1;
    }
}

#[cfg(test)]
#[path = "palette_test.rs"]
mod tests;
