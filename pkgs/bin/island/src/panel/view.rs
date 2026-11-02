use gpui::{AnyElement, Context, FontWeight, SharedString, div, prelude::*, px, relative, rgb};

use super::{
    block::render_all,
    frame::Frame,
    model::{Block, Button, Header, HeaderLead, PanelView, Views},
};
use crate::{
    components::{
        badge, button, card, dropdown, edges, icon, motion,
        overscroll::overscroll,
        tabs,
        tooltip::{Tip, Tooltips},
    },
    island::{Island, Mode},
    panel::IslandUi,
    theme::{OVERLAY2, SUBTEXT1, TEXT},
    ui::{
        self, BLOCK_GAP, HEADER_GAP, HEADER_H, HEADING_SIZE, INSET, SMALL_SIZE, TIGHT_INSET,
        TITLE_SIZE,
    },
};

/// Between two views as they pass: wide enough that the leaving one is past the island's edge
/// before the spring slows at the end, or its edge lingers in the panel's padding.
const PASSING_GAP: f32 = 96.0;

const CAPTION_SIZE: f32 = 12.0;

/// How near its end a followed panel counts as being read there.
pub(super) const FOLLOW_SLACK: f32 = 24.0;

impl PanelView {
    pub fn render(mut self, frame: &Frame<'_>, cx: &mut Context<Island>) -> AnyElement {
        let inset = match self.header {
            Header::None => TIGHT_INSET,
            _ => INSET,
        };
        let passing = match &mut self.header {
            Header::Views(views) => views.passing.take(),
            _ => None,
        };
        // A conversation keeps its newest text in view while you read at its end; scrolled back
        // to read something earlier, it stays where you are.
        if self.blocks.iter().any(Block::follows_end) {
            let (offset, end) = (frame.scroll.offset().y, frame.scroll.max_offset().y);
            if f32::from(end + offset).abs() < FOLLOW_SLACK {
                frame.scroll.scroll_to_bottom();
            }
        }
        let header = match self.header {
            Header::Tabs => Some(
                div()
                    .h(px(HEADER_H))
                    .min_w(px(crate::bar::tabs_width()))
                    .into_any_element(),
            ),
            Header::Title(title, buttons) => {
                Some(title_header(title, buttons, frame.hover, frame.tips, cx))
            }
            Header::Views(views) => Some(views_header(views, frame.hover, frame.tips, cx)),
            Header::None => None,
        };
        let island_below = frame.island_h - inset;
        // An input, with whatever follows it, stays at the panel's foot below what scrolls, as a
        // chat's message field does.
        let mut blocks = self.blocks;
        let prompt = blocks
            .iter()
            .rposition(|block| matches!(block, Block::Prompt(_)))
            .map(|at| blocks.remove(at).render("prompt", island_below, frame, cx));
        let foot = blocks
            .iter()
            .rposition(|block| matches!(block, Block::Input(_)))
            .map_or_else(Vec::new, |at| blocks.split_off(at));
        let foot = render_all(foot, "foot", island_below, frame, cx);
        // A page's own views stay in place while what they show rises in under them.
        let page_views = matches!(blocks.first(), Some(Block::Views(_))).then(|| {
            blocks
                .remove(0)
                .render("page-views", island_below, frame, cx)
        });
        let blocks = render_all(blocks, "block", island_below, frame, cx);
        let content = {
            let column = div()
                .flex()
                .flex_col()
                .gap(px(BLOCK_GAP))
                .children(blocks)
                .into_any_element();
            match page_views {
                Some(views) => div()
                    .flex()
                    .flex_col()
                    .gap(px(BLOCK_GAP))
                    .child(views)
                    .child(column)
                    .into_any_element(),
                None => column,
            }
        };
        let content = match passing {
            Some(passing) => {
                let side = if passing.rightwards { 1.0 } else { -1.0 };
                let room = frame.island_w - 2.0 * inset;
                let width = passing.width.get().unwrap_or(room);
                passing.width.set(Some(width));
                let travel = width.max(room) + PASSING_GAP;
                let leaving = render_all(passing.blocks, "passing", island_below, frame, cx);
                div()
                    .relative()
                    .child(
                        div()
                            .relative()
                            .left(px(((1.0 - passing.at) * side * travel).round()))
                            .child(content),
                    )
                    .child(
                        div()
                            .absolute()
                            .top_0()
                            .left(relative(0.5))
                            .ml(px((-width / 2.0 - passing.at * side * travel).round()))
                            .w(px(width))
                            .flex()
                            .flex_col()
                            .gap(px(BLOCK_GAP))
                            .children(leaving),
                    )
                    .into_any_element()
            }
            None => content,
        };
        let body = div().id("panel-body");
        let body = if self.scroll {
            edges::edges(
                overscroll(
                    "panel-body",
                    body.flex_initial().min_h(px(0.0)).overflow_y_scroll(),
                    Some(frame.scroll),
                    content,
                ),
                frame.scroll,
                frame.glass,
            )
        } else {
            body.child(content).flex_none().into_any_element()
        };
        ui::base()
            .relative()
            .flex()
            .flex_col()
            .min_w(px(frame.min_width))
            .max_w(px(frame.max_width))
            .when(self.scroll, |panel| panel.max_h(px(frame.max_height)))
            .p(px(inset))
            .when_some(header, |panel, header| {
                panel
                    .child(header)
                    .child(div().h(px(HEADER_GAP)).flex_none())
            })
            .child(body)
            .when(!foot.is_empty(), |panel| {
                panel.child(div().h(px(BLOCK_GAP)).flex_none()).child(
                    div()
                        .flex_none()
                        .flex()
                        .flex_col()
                        .gap(px(BLOCK_GAP))
                        .children(foot),
                )
            })
            .children(prompt)
            .into_any_element()
    }
}

fn title_header(
    title: SharedString,
    buttons: Vec<Button>,
    hover: bool,
    tips: &Tooltips,
    cx: &mut Context<Island>,
) -> AnyElement {
    let title = div()
        .min_w(px(0.0))
        .truncate()
        .text_size(px(HEADING_SIZE))
        .font_weight(FontWeight::SEMIBOLD)
        .child(motion::text("panel-title", title));
    header_row(
        title.into_any_element(),
        false,
        None,
        buttons,
        hover,
        tips,
        cx,
    )
}

fn views_header(
    views: Views,
    hover: bool,
    tips: &Tooltips,
    cx: &mut Context<Island>,
) -> AnyElement {
    let tabs = tabs::tabs(
        "panel-views",
        views.tabs,
        views.selected,
        views.on_select,
        hover,
        tips,
        cx,
    );
    let lead = views.lead.map(|lead| header_lead(lead, cx));
    header_row(
        tabs.into_any_element(),
        true,
        lead,
        views.buttons,
        hover,
        tips,
        cx,
    )
}

const STRIP_DIVIDER_H: f32 = 18.0;

/// The views at the start, in a strip of their own, and their buttons at the end, in a page rather
/// than its header, so without the close button.
pub(super) fn views_row(views: Views, frame: &Frame<'_>, cx: &mut Context<Island>) -> AnyElement {
    let hover = frame.hover;
    let tabs = tabs::strip(
        "page-views",
        views.tabs,
        views.selected,
        views.on_select,
        hover,
        frame.layouts,
        cx,
    );
    let pick = views
        .pick
        .map(|pick| dropdown::dropdown(pick, frame.popovers, hover, cx));
    let groups = views.icons.len();
    let mut strip = Vec::new();
    for (index, group) in views.icons.into_iter().enumerate() {
        for button in group {
            let handler = button.on_click;
            strip.push(
                button::strip_icon(
                    button.id.clone(),
                    button.glyph.as_deref(),
                    button.color,
                    button.active,
                    hover,
                )
                .tip(frame.tips, button.id, button.label)
                .on_click(cx.listener(move |this, _, _, cx| handler(&mut IslandUi::new(this, cx))))
                .into_any_element(),
            );
        }
        if index + 1 < groups {
            strip.push(ui::strip_divider(STRIP_DIVIDER_H).into_any_element());
        }
    }
    let icons = (!strip.is_empty()).then(|| {
        div()
            .relative()
            .flex_none()
            .p(px(ui::STRIP_PAD))
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full()
                    .child(ui::strip()),
            )
            .child(
                div()
                    .relative()
                    .flex()
                    .items_center()
                    .gap(px(2.0))
                    .children(strip),
            )
    });
    div()
        .min_h(px(HEADER_H))
        .flex()
        .items_center()
        .justify_between()
        .gap(px(16.0))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(12.0))
                .child(tabs)
                .children(pick),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .children(
                    views
                        .buttons
                        .into_iter()
                        .map(|button| button_element(button, hover, frame.tips, cx)),
                )
                .children(icons),
        )
        .into_any_element()
}

fn header_lead(lead: HeaderLead, cx: &mut Context<Island>) -> AnyElement {
    let mark = div()
        .id("panel-lead-mark")
        .size(px(lead.size))
        .flex_none()
        .child(badge::mark(&lead.mark, lead.size, 1.0))
        .when_some(lead.on_click, |mark, handler| {
            // The mark answers the pointer itself, so it goes without the grey hover behind.
            mark.cursor_pointer()
                .on_click(cx.listener(move |this, _, _, cx| handler(&mut IslandUi::new(this, cx))))
        });
    div()
        .min_w(px(0.0))
        .flex()
        .items_center()
        .gap(px(10.0))
        .child(mark)
        .child(
            div()
                .min_w(px(0.0))
                .flex()
                .flex_col()
                .child(
                    div()
                        .truncate()
                        .text_size(px(SMALL_SIZE))
                        .text_color(rgb(OVERLAY2))
                        .child(lead.overline),
                )
                .child(
                    div()
                        .truncate()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(motion::text("panel-lead-title", lead.title)),
                ),
        )
        .into_any_element()
}

/// `centred` puts `leading` in the middle, with equal room either side, as the bar's tabs sit,
/// and `lead` at the start of that room.
fn header_row(
    leading: AnyElement,
    centred: bool,
    lead: Option<AnyElement>,
    buttons: Vec<Button>,
    hover: bool,
    tips: &Tooltips,
    cx: &mut Context<Island>,
) -> AnyElement {
    div()
        .min_h(px(HEADER_H))
        .flex_none()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(16.0))
        .when(centred, |row| {
            row.child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .flex()
                    .items_center()
                    .children(lead),
            )
        })
        .child(leading)
        .child(
            div()
                .flex()
                // Never narrower than its buttons, so a tight panel moves the tabs aside rather
                // than drawing the buttons over them.
                .when(centred, |end| end.flex_1().flex_shrink_0().justify_end())
                .when(!centred, |end| end.flex_none())
                .items_center()
                .gap(px(6.0))
                .children(
                    buttons
                        .into_iter()
                        .map(|button| button_element(button, hover, tips, cx)),
                )
                .child(
                    button::close("panel-close", hover)
                        .on_click(cx.listener(|this, _, _, cx| this.set_mode(Mode::Collapsed, cx))),
                ),
        )
        .into_any_element()
}

fn button_element(
    button: Button,
    hover: bool,
    tips: &Tooltips,
    cx: &mut Context<Island>,
) -> AnyElement {
    let handler = button.on_click;
    let hover = hover && button.enabled;
    let has_glyph = button.glyph.is_some();
    let tip_id = button.id.clone();
    let tip_label = button.label.clone();
    let mut spec = button::Button::new(button.id)
        .glyph(button.glyph.as_deref())
        .bare(button.bare)
        .active(button.active)
        .toggle(button.toggle)
        .fits(button.fits)
        .busy(button.busy);
    if !has_glyph {
        spec = spec.label(button.label);
    }
    let element = button::button(spec, hover).when(has_glyph && !tip_label.is_empty(), |el| {
        el.tip(tips, tip_id, tip_label)
    });
    if button.enabled {
        element
            .on_click(cx.listener(move |this, _, _, cx| handler(&mut IslandUi::new(this, cx))))
            .into_any_element()
    } else {
        element
            .opacity(0.5)
            .text_color(rgb(OVERLAY2))
            .cursor_default()
            .into_any_element()
    }
}

const TITLE_GLYPH: f32 = 16.0;
const OPEN_SIZE: f32 = 24.0;

pub(super) fn card_title(
    title: card::Title,
    at: &str,
    hover: bool,
    cx: &mut Context<Island>,
) -> AnyElement {
    let open = title.open.map(|handler| {
        button::ghost(SharedString::from(format!("{at}-open")), hover)
            .flex_none()
            .size(px(OPEN_SIZE))
            .text_size(px(TITLE_SIZE))
            .text_color(rgb(SUBTEXT1))
            .when(hover, |open| {
                open.hover(|style| style.text_color(rgb(TEXT)))
            })
            .child("›")
            .on_click(cx.listener(move |this, _, _, cx| handler(&mut IslandUi::new(this, cx))))
    });
    let centred = title.glyph.is_some() || open.is_some();
    div()
        .flex()
        .when(centred, |line| line.items_center())
        .when(!centred, |line| line.items_baseline().justify_between())
        .gap(px(if centred { 8.0 } else { 12.0 }))
        .children(
            title
                .glyph
                .map(|(path, color)| icon::glyph(&path, TITLE_GLYPH, color)),
        )
        .child(
            div()
                .when(centred, |text| text.flex_1().min_w(px(0.0)).truncate())
                .text_size(px(TITLE_SIZE))
                .font_weight(FontWeight::SEMIBOLD)
                .child(motion::text(
                    SharedString::from(format!("{at}-title")),
                    title.text,
                )),
        )
        .when(!title.detail.is_empty(), |line| {
            line.child(
                div()
                    .min_w(px(0.0))
                    .truncate()
                    .text_size(px(CAPTION_SIZE))
                    .text_color(rgb(OVERLAY2))
                    .child(motion::text(
                        SharedString::from(format!("{at}-detail")),
                        title.detail,
                    )),
            )
        })
        .children(open)
        .into_any_element()
}
