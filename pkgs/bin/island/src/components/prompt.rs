//! A small card over the panel asking for one thing, such as a network's password or whether a
//! code matches the one a device shows. The panel stays in view under it, dimmed, and takes no
//! clicks until the card is answered or put away.

use std::path::PathBuf;

use gpui::{AnyElement, Context, FontWeight, SharedString, div, prelude::*, px, rgb, rgba};

use crate::{
    components::{
        button,
        input::{self, InputView},
        popover, reveal,
    },
    island::Island,
    panel::Handler,
    theme::{CRUST, OVERLAY2, RED, TINT, with_alpha},
    ui::{self, Radius, TITLE_SIZE},
};

const CAPTION_SIZE: f32 = 12.0;
const WIDTH: f32 = 340.0;
const PAD: f32 = 16.0;
const SCRIM_ALPHA: u8 = 0x99;

/// A switch between dots and the text itself.
pub struct Reveal {
    pub glyph: Option<PathBuf>,
    pub shown: bool,
    pub on_click: Handler,
}

pub struct PromptView {
    pub id: SharedString,
    pub title: SharedString,
    pub detail: SharedString,
    /// Its send button answers the card.
    pub field: Option<InputView>,
    pub reveal: Option<Reveal>,
    /// Shown large, such as a pairing code to compare or type.
    pub code: Option<SharedString>,
    /// A button answering the card, beside the way back, such as Confirm.
    pub confirm: Option<(SharedString, Handler)>,
    pub error: Option<SharedString>,
    pub on_cancel: Handler,
}

const CODE_SIZE: f32 = 30.0;

pub fn prompt(view: PromptView, hover: bool, cx: &mut Context<Island>) -> AnyElement {
    let cancel = view.on_cancel.clone();
    let eye = view.reveal.map(|reveal| {
        let handler = reveal.on_click;
        button::icon(
            SharedString::from(format!("{}-reveal", view.id)),
            reveal.glyph.as_deref(),
            None,
            reveal.shown,
            hover,
        )
        .on_click(cx.listener(move |island, _, _, cx| {
            cx.stop_propagation();
            handler(island, cx);
        }))
    });
    let back = view.on_cancel;
    let card = popover::surface(Radius::Panel).w(px(WIDTH)).child(
        div()
            .id(SharedString::from(format!("{}-card", view.id)))
            .relative()
            .p(px(PAD))
            .flex()
            .flex_col()
            .gap(px(10.0))
            .on_click(|_, _, cx| cx.stop_propagation())
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .child(
                        div()
                            .truncate()
                            .text_size(px(TITLE_SIZE))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(view.title),
                    )
                    .when(!view.detail.is_empty(), |text| {
                        text.child(
                            div()
                                .text_size(px(CAPTION_SIZE))
                                .text_color(rgb(OVERLAY2))
                                .child(view.detail),
                        )
                    }),
            )
            .children(view.code.map(|code| {
                div()
                    .relative()
                    .py(px(14.0))
                    .flex()
                    .justify_center()
                    .child(ui::shape(
                        Radius::Control,
                        Some(rgba(with_alpha(TINT, 0x0d)).into()),
                    ))
                    .child(
                        div()
                            .relative()
                            .text_size(px(CODE_SIZE))
                            .font_weight(FontWeight::BOLD)
                            .child(code),
                    )
            }))
            .children(view.field.map(|field| {
                div()
                    .flex()
                    .items_end()
                    .gap(px(8.0))
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.0))
                            .child(input::input(field, hover, cx)),
                    )
                    .children(eye)
            }))
            .children(view.error.map(|error| {
                div()
                    .text_size(px(CAPTION_SIZE))
                    .text_color(rgb(RED))
                    .child(error)
            }))
            .child(
                div()
                    .flex()
                    .justify_end()
                    .gap(px(8.0))
                    .child(
                        button::button(
                            SharedString::from(format!("{}-cancel", view.id)),
                            "Cancel",
                            false,
                            hover,
                        )
                        .on_click(cx.listener(move |island, _, _, cx| {
                            cx.stop_propagation();
                            back(island, cx);
                        })),
                    )
                    .children(view.confirm.map(|(label, handler)| {
                        button::button(
                            SharedString::from(format!("{}-confirm", view.id)),
                            label,
                            true,
                            hover,
                        )
                        .on_click(cx.listener(move |island, _, _, cx| {
                            cx.stop_propagation();
                            handler(island, cx);
                        }))
                    })),
            ),
    );
    div()
        .id(view.id.clone())
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .occlude()
        .flex()
        .items_center()
        .justify_center()
        .bg(rgba(with_alpha(CRUST, SCRIM_ALPHA)))
        .on_click(cx.listener(move |island, _, _, cx| cancel(island, cx)))
        .child(reveal::appear(
            SharedString::from(format!("{}-appear", view.id)),
            card,
        ))
        .into_any_element()
}
