//! A grid of picture cards to pick one from, with keys or the pointer, such as windows to switch
//! to: each card a picture over a mark, a name and a detail; the picked one under a highlight that
//! slides between them; the picked one's whole caption beneath, wrapping as it needs; and what the
//! keys do. Past a few rows the grid scrolls to keep what is picked in view, its edges fading.

use crate::{
    components::{badge::Mark, search::Scroll},
    panel::{Handler, SharedString},
    picture::Picture,
};

pub const COLUMNS: usize = 5;
/// Rows in view before the grid scrolls.
pub const ROWS: usize = 3;
pub const CARD_W: f32 = 256.0;
pub const CARD_PAD: f32 = 10.0;
pub const CARD_GAP: f32 = 4.0;
/// Between the grid, the caption and the keys.
pub const GAP: f32 = 10.0;
pub const PICTURE_W: f32 = CARD_W - 2.0 * CARD_PAD;
pub const PICTURE_H: f32 = 133.0;
pub const MARK: f32 = 30.0;
pub const MARK_GAP: f32 = 12.0;
/// The room a card's name has beside its mark.
pub const NAME_W: f32 = PICTURE_W - MARK - MARK_GAP;

pub struct GalleryCard {
    /// Names it across frames, so it moves rather than appears as cards come and go.
    pub key: SharedString,
    /// Shown fitted in its place; without it, the mark shows there larger.
    pub picture: Option<Picture>,
    pub mark: Mark,
    pub name: SharedString,
    /// Lines the name takes, as its owner measured it at [`NAME_W`].
    pub lines: usize,
    pub detail: SharedString,
    /// A dot before the detail in this color, such as something wanting attention.
    pub flag: Option<u32>,
    /// Its place in the grid, counted along the rows.
    pub slot: usize,
    /// 1 while it is in the grid, going to 0 as it leaves its place.
    pub presence: f32,
    pub on_click: Option<Handler>,
    pub on_hover: Option<Handler>,
}

pub struct GalleryView {
    /// Back to front, so cards leaving can be drawn under the ones staying.
    pub cards: Vec<GalleryCard>,
    /// How many places the cards staying fill.
    pub places: usize,
    /// The place picked.
    pub picked: Option<usize>,
    /// The first row in view, once there are more rows than fit.
    pub first_row: usize,
    /// What the picked card is in full, such as a window's title.
    pub caption: SharedString,
    /// Lines the caption takes, as its owner measured it at [`width`].
    pub caption_lines: usize,
    /// Shown when there are no cards.
    pub empty: SharedString,
    /// Told how many rows the wheel turned over it.
    pub on_rows: Scroll,
    /// What each key does, as a line under the caption, such as `("Tab", "next")`.
    pub keys: Vec<(SharedString, SharedString)>,
}

/// How many columns `places` cards take.
pub const fn columns(places: usize) -> usize {
    if places < 1 {
        1
    } else if places > COLUMNS {
        COLUMNS
    } else {
        places
    }
}

/// How wide the grid is with `places` cards; the caption wraps to this.
pub const fn width(places: usize) -> f32 {
    let columns = columns(places);
    columns as f32 * CARD_W + (columns - 1) as f32 * CARD_GAP
}
