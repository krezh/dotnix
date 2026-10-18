use gpui::SharedString;

use super::{
    entry::Kind,
    runtime::{Clipboard, Request},
    state::State,
};
use crate::{
    components::list::{self, ROWS, Selection},
    features::PanelCx,
    island::Mode,
    panel::{Block, Header, PanelView, Preview, SearchRow, SearchView},
};

pub(super) fn panel(state: &State, p: &PanelCx<'_>) -> Option<PanelView> {
    let picker = p.local::<Selection>()?;
    let matching = state.matching(&picker.query);
    let count = matching.len();
    let selected = picker.selected.min(count.saturating_sub(1));
    let top = list::reveal(selected, picker.top, count);
    let rows = matching
        .iter()
        .enumerate()
        .skip(top)
        .take(ROWS)
        .map(|(position, entry)| {
            let id = entry.id;
            let glyph = match entry.kind {
                Kind::Text => "clipboard-text",
                Kind::Image => "clipboard-image",
                Kind::Files => "clipboard-files",
            };
            SearchRow {
                id: format!("clip-{id}").into(),
                name: entry.title().into(),
                detail: entry.detail().into(),
                preview: entry.preview.is_some(),
                icon: entry.preview.clone().or_else(|| p.glyph(glyph)),
                selected: position == selected,
                on_click: p.on_island::<Clipboard>(move |clipboard, island, cx| {
                    clipboard.request(Request::Select { id, paste: true });
                    island.set_mode(Mode::Collapsed, cx);
                }),
                on_hover: p.on_local::<Clipboard>(move |_, picker| picker.select(position, count)),
            }
        })
        .collect();
    let preview = match matching.get(selected) {
        Some(entry) => (entry.preview(), entry.about().into()),
        None => (Preview::Empty, SharedString::default()),
    };
    Some(PanelView {
        header: Header::None,
        blocks: vec![Block::Search(SearchView {
            query: picker.query.clone().into(),
            placeholder: "Search the clipboard".into(),
            empty: state
                .error
                .clone()
                .unwrap_or_else(|| "Nothing copied yet".to_owned())
                .into(),
            rows,
            on_step: p
                .on_local_with::<Clipboard, i32>(move |_, picker, step| picker.step(step, count)),
            preview: Some(preview),
        })],
    })
}
