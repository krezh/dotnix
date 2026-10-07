const MAX_WIDTH: f64 = 760.0;
const OUTER_MARGIN: f64 = 24.0;
const NARROW_MARGIN: f64 = 12.0;
const PADDING: f64 = 16.0;
const HEADING_HEIGHT: f64 = 16.0;
const HEADING_GAP: f64 = 8.0;
const ITEM_GAP: f64 = 10.0;
const SECTION_GAP: f64 = 16.0;
const MIN_ITEM_WIDTH: f64 = 132.0;
const MAX_ITEMS: usize = 12;

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
    SaveReplay,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ReplayPaletteState {
    pub visible: bool,
    pub can_save: bool,
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
    pub replay_heading_y: f64,
    items: [PaletteItem; MAX_ITEMS],
    item_count: usize,
}

impl ModePaletteLayout {
    pub fn new(
        width: i32,
        height: i32,
        control_height: u32,
        is_recording: bool,
        supports_window_capture: bool,
        replay: ReplayPaletteState,
    ) -> Self {
        let screen_width = f64::from(width.max(1));
        let screen_height = f64::from(height.max(1));
        let margin = if screen_width < 400.0 || screen_height < 320.0 {
            8.0
        } else if screen_width < 480.0 {
            NARROW_MARGIN
        } else {
            OUTER_MARGIN
        };
        let max_panel_width = (screen_width - margin * 2.0).max(1.0);
        let max_panel_height = (screen_height - margin * 2.0).max(1.0);
        let panel_width = max_panel_width.min(MAX_WIDTH);

        let mut padding = if panel_width < 360.0 || max_panel_height < 360.0 {
            10.0
        } else {
            PADDING
        };
        if panel_width - padding * 2.0 < 80.0 {
            padding = 4.0;
        }
        let content_width = (panel_width - padding * 2.0).max(1.0);

        let mut item_gap = if content_width < 360.0 { 6.0 } else { ITEM_GAP };
        let mut heading_gap = HEADING_GAP;
        let mut heading_height = HEADING_HEIGHT;
        let mut section_gap = if max_panel_height < 400.0 {
            10.0
        } else {
            SECTION_GAP
        };

        let min_item_width = if max_panel_height < 320.0 {
            40.0
        } else {
            MIN_ITEM_WIDTH
        };
        let columns = ((content_width + item_gap) / (min_item_width + item_gap))
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
        let replay_count: usize = if replay.visible && replay.can_save {
            1
        } else {
            0
        };

        let capture_rows = capture_count.div_ceil(columns);
        let record_columns = if is_recording { 1 } else { columns.min(3) };
        let record_rows = record_count.div_ceil(record_columns);
        let replay_rows = replay_count;
        let total_rows = capture_rows + record_rows + replay_rows;

        let mut item_height = (f64::from(control_height) * 0.75).clamp(36.0, 48.0);

        let mut sections_count = 2;
        if replay.visible {
            sections_count += 1;
        }
        let needed_section_gaps = (sections_count - 1) as f64 * section_gap;
        let needed_headings = sections_count as f64 * heading_height
            + (capture_rows > 0) as usize as f64 * heading_gap
            + (record_rows > 0) as usize as f64 * heading_gap
            + (replay_rows > 0) as usize as f64 * heading_gap;
        let needed_item_gaps = item_gap
            * (capture_rows.saturating_sub(1)
                + record_rows.saturating_sub(1)
                + replay_rows.saturating_sub(1)) as f64;

        let needed_overhead =
            padding * 2.0 + needed_section_gaps + needed_headings + needed_item_gaps;
        let unscaled_height = needed_overhead + item_height * total_rows as f64;

        if unscaled_height > max_panel_height {
            let available_for_rows =
                (max_panel_height - needed_overhead).max(total_rows as f64 * 20.0);
            item_height = (available_for_rows / total_rows as f64).clamp(18.0, 48.0);

            let second_check = padding * 2.0
                + needed_section_gaps
                + needed_headings
                + needed_item_gaps
                + item_height * total_rows as f64;
            if second_check > max_panel_height {
                let compression = (max_panel_height / second_check).clamp(0.4, 1.0);
                padding = (padding * compression).max(4.0);
                heading_height = (heading_height * compression).max(10.0);
                heading_gap = (heading_gap * compression).max(2.0);
                section_gap = (section_gap * compression).max(4.0);
                item_gap = (item_gap * compression).max(3.0);
                item_height = (item_height * compression).max(16.0);
            }
        }

        let panel_height = (padding * 2.0
            + (sections_count - 1) as f64 * section_gap
            + sections_count as f64 * heading_height
            + (capture_rows > 0) as usize as f64 * heading_gap
            + (record_rows > 0) as usize as f64 * heading_gap
            + (replay_rows > 0) as usize as f64 * heading_gap
            + item_height * total_rows as f64
            + item_gap
                * (capture_rows.saturating_sub(1)
                    + record_rows.saturating_sub(1)
                    + replay_rows.saturating_sub(1)) as f64)
            .min(max_panel_height);

        let x = (screen_width - panel_width) / 2.0;
        let y = (screen_height - panel_height - margin)
            .clamp(margin, (screen_height - panel_height).max(0.0));

        let capture_heading_y = y + padding;
        let capture_y = capture_heading_y + heading_height + heading_gap;
        let record_heading_y = capture_y
            + item_height * capture_rows as f64
            + item_gap * capture_rows.saturating_sub(1) as f64
            + section_gap;
        let record_y = record_heading_y + heading_height + heading_gap;
        let replay_heading_y = if replay.visible {
            record_y
                + item_height * record_rows as f64
                + item_gap * record_rows.saturating_sub(1) as f64
                + section_gap
        } else {
            0.0
        };
        let replay_y = replay_heading_y + heading_height + heading_gap;

        let mut layout = Self {
            bounds: PaletteRect {
                x,
                y,
                width: panel_width,
                height: panel_height,
            },
            capture_heading_y,
            record_heading_y,
            replay_heading_y,
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
                padding,
                item_gap,
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
                padding,
                item_gap,
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
                    padding,
                    item_gap,
                );
            }
        }

        if replay.visible && replay.can_save {
            layout.push_grid_item(
                PaletteAction::SaveReplay,
                0,
                1,
                1,
                replay_y,
                content_width,
                item_height,
                padding,
                item_gap,
            );
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
        padding: f64,
        item_gap: f64,
    ) {
        let row = index / columns;
        let column = index % columns;
        let items_in_row = (item_count - row * columns).min(columns);
        let item_width = (content_width - item_gap * items_in_row.saturating_sub(1) as f64)
            / items_in_row as f64;
        let row_width =
            item_width * items_in_row as f64 + item_gap * items_in_row.saturating_sub(1) as f64;
        let row_x = self.bounds.x + padding + (content_width - row_width) / 2.0;

        self.items[self.item_count] = PaletteItem {
            action,
            rect: PaletteRect {
                x: row_x + column as f64 * (item_width + item_gap),
                y: y + row as f64 * (item_height + item_gap),
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
