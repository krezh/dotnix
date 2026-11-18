//! Cards stacked down a list, each at its own place as the list moves: sliding and fading in and
//! out, the edges of cards folded behind one peeking out below it, a note while there are none and
//! a count of those left out kept to the island's bottom edge; two lists passing side by side, and
//! lines opening over a list.

use crate::{
    panel::{Block, FrameView, Place, Placed, SharedString},
    ui::TIGHT_INSET,
};

use super::card::{Look, Motion};

/// Between one card and the next.
pub const GAP: f32 = 8.0;
/// The count of those left out, under the cards.
pub const FOOTER_H: f32 = 18.0;
/// How tall each edge showing below a front card is, nearest first: one for each card folded
/// behind it, up to two.
pub const PEEKS: [f32; 2] = [8.0, 6.0];
/// How far in from the front card's sides each edge sits, and how strongly it shows, nearest first.
const PEEK_INSETS: [f32; 2] = [12.0, 26.0];
const PEEK_OPACITY: [f32; 2] = [1.0, 0.6];
/// The frame each edge is the bottom of, tall enough for its rounded corners.
const PEEK_DRAWN: f32 = 40.0;
const EMPTY_H: f32 = 52.0;
/// Between two lists as they pass: wide enough that the leaving one is past the island's edge
/// before the spring slows at the end, or its edge lingers in the panel's padding.
const ACROSS_GAP: f32 = 96.0;
/// How tall each line over a list is.
const ABOVE_H: f32 = 28.0;

pub struct Card {
    pub y: f32,
    pub height: f32,
    pub opacity: f32,
    pub peeks: Option<Peeks>,
    pub block: Block,
}

pub struct Peeks {
    pub id: SharedString,
    pub count: usize,
    /// How far out they are: 1 while folded, 0 once spread out.
    pub out: f32,
    pub opacity: f32,
}

pub struct Note {
    pub text: SharedString,
    pub opacity: f32,
}

pub struct Scroll {
    pub visible: f32,
    pub still: f32,
}

pub struct Stack {
    pub width: f32,
    /// Back to front.
    pub cards: Vec<Card>,
    pub height: f32,
    pub scroll: Option<Scroll>,
    pub empty: Option<Note>,
    pub footer: Option<Note>,
}

fn place(y: f32, height: f32, opacity: f32) -> Place {
    Place {
        inset: 0.0,
        offset: 0.0,
        y,
        height,
        opacity,
        from_bottom: false,
        drawn: None,
    }
}

/// The edges below a card whose bottom is at `bottom`, nearest first.
fn peeks(peeks: &Peeks, bottom: f32) -> Vec<(Place, Block)> {
    let mut y = bottom;
    PEEKS
        .iter()
        .zip(PEEK_INSETS)
        .zip(PEEK_OPACITY)
        .take(peeks.count)
        .enumerate()
        .map(|(index, ((height, inset), strength))| {
            let height = height * peeks.out;
            let at = Place {
                inset,
                offset: 0.0,
                y,
                height,
                opacity: peeks.opacity * strength * peeks.out,
                from_bottom: false,
                drawn: Some(PEEK_DRAWN),
            };
            y += height;
            let frame = FrameView {
                id: format!("{}-peek-{index}", peeks.id).into(),
                look: Look::Raised { urgent: false },
                on_click: None,
                on_dismiss: None,
                countdown: None,
                selected: false,
                motion: Motion::Still,
                shown_height: None,
            };
            (at, Block::Frame(frame, Vec::new()))
        })
        .collect()
}

/// The edges of those folded behind a card are drawn before it, so it covers them.
pub fn stack(stack: Stack) -> Block {
    let mut blocks = Vec::new();
    for card in stack.cards {
        if card.opacity <= 0.001 {
            continue;
        }
        if let Some(behind) = card.peeks.as_ref().filter(|behind| behind.count > 0)
            && behind.out > 0.001
        {
            blocks.extend(peeks(behind, card.y + card.height));
        }
        blocks.push((place(card.y, card.height, card.opacity), card.block));
    }
    let empty = stack.empty.is_some();
    if let Some(note) = stack.empty {
        blocks.push((place(0.0, EMPTY_H, note.opacity), Block::note(note.text)));
    }
    if let Some(footer) = stack.footer.filter(|footer| footer.opacity > 0.001) {
        let at = Place {
            inset: 0.0,
            offset: 0.0,
            y: TIGHT_INSET,
            height: FOOTER_H,
            opacity: footer.opacity,
            from_bottom: true,
            drawn: None,
        };
        blocks.push((at, Block::note(footer.text)));
    }
    let (height, visible, still) = match stack.scroll {
        None if !empty => (stack.height, None, stack.height),
        scroll => {
            let (visible, still) = match scroll {
                Some(scroll) if !empty => (scroll.visible, scroll.still),
                _ => (EMPTY_H, EMPTY_H),
            };
            (stack.height.max(visible), Some(visible), still.max(visible))
        }
    };
    Block::Placed(Placed {
        width: stack.width,
        height,
        visible,
        still,
        rewind: None,
        blocks,
    })
}

pub fn height(heights: &[f32], footer: bool) -> f32 {
    let cards: f32 = heights.iter().sum::<f32>() + GAP * heights.len().saturating_sub(1) as f32;
    let footer = if footer { GAP + FOOTER_H } else { 0.0 };
    cards + footer
}

/// `left` moving out as `right` moves in, `at` of the way across, as their tabs sit side by side;
/// the other way round unless `rightwards`. Both stay whole, so nothing blanks or shows through
/// the other on the way, and the one leaving scrolls back to the top as it goes, so the one coming
/// in arrives at its top.
pub fn across(left: Block, right: Block, at: f32, rightwards: bool) -> Block {
    let (mut left, mut right) = match (left, right) {
        (Block::Placed(left), Block::Placed(right)) => (left, right),
        (_, right) => return right,
    };
    let width = left.width.max(right.width);
    let travel = width + ACROSS_GAP;
    let (leaving, coming, left_to_go) = if rightwards {
        (&left, &right, 1.0 - at)
    } else {
        (&right, &left, at)
    };
    let out_of_view = |list: &Placed| (list.still - list.visible.unwrap_or(list.still)).max(0.0);
    let rewind = Some(out_of_view(leaving) * left_to_go);
    let coming_out_of_view = out_of_view(coming);
    for (place, _) in &mut left.blocks {
        place.offset -= at * travel;
    }
    for (place, _) in &mut right.blocks {
        place.offset += (1.0 - at) * travel;
    }
    let visible = left
        .visible
        .zip(right.visible)
        .map(|(left, right)| left + (right - left) * at);
    left.blocks.append(&mut right.blocks);
    Block::Placed(Placed {
        width,
        height: left.height.max(right.height),
        visible,
        // Its edges are the coming one's from the start, so they are there as it comes in.
        still: visible.unwrap_or(0.0) + coming_out_of_view,
        rewind,
        blocks: left.blocks,
    })
}

/// `above` opening over `list`, `open` of the way out, the list moving down to make room.
pub fn opening(list: Block, above: Vec<Block>, open: f32) -> Vec<Block> {
    let open = open.clamp(0.0, 1.0);
    if above.is_empty() || open <= 0.001 {
        return vec![list];
    }
    let Block::Placed(mut placed) = list else {
        let mut blocks = above;
        blocks.push(list);
        return blocks;
    };
    let room = (above.len() as f32 * ABOVE_H + GAP) * open;
    for (place, _) in &mut placed.blocks {
        if !place.from_bottom {
            place.y += room;
        }
    }
    let lines = above.into_iter().enumerate().map(|(index, block)| {
        let at = Place {
            inset: 0.0,
            offset: 0.0,
            y: index as f32 * ABOVE_H * open,
            height: ABOVE_H * open,
            opacity: open,
            from_bottom: false,
            drawn: Some(ABOVE_H),
        };
        (at, block)
    });
    placed.blocks.splice(0..0, lines);
    placed.height += room;
    placed.still += room;
    placed.visible = placed.visible.map(|visible| visible + room);
    vec![Block::Placed(placed)]
}

#[cfg(test)]
#[path = "stack_test.rs"]
mod tests;
