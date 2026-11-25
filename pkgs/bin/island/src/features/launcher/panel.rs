use super::{runtime::Launcher, state::Local};
use crate::{
    components::list::ROWS,
    features::PanelCx,
    panel::{Block, Header, PanelView, RowIcon, SearchRow, SearchView},
};

pub(super) fn panel(p: &PanelCx<'_>) -> Option<PanelView> {
    let local = p.local::<Local>()?;
    let search = &local.selection;
    let rows = local
        .matching()
        .enumerate()
        .skip(search.top)
        .take(ROWS)
        .map(|(position, app)| {
            let picked = app.clone();
            SearchRow {
                name: app.name.clone().into(),
                detail: if app.generic.is_empty() {
                    &app.comment
                } else {
                    &app.generic
                }
                .clone()
                .into(),
                icon: app.icon.clone().map_or(RowIcon::None, RowIcon::Image),
                mono: false,
                section: None,
                mark: None,
                trail: Vec::new(),
                selected: position == search.selected,
                on_click: p.on_island::<Launcher>(move |launcher, ui| {
                    launcher.launch(&picked);
                    ui.collapse();
                }),
                on_hover: p.on_local::<Launcher>(move |_, local| local.select(position)),
            }
        })
        .collect();
    Some(PanelView {
        scroll: false,
        header: Header::None,
        blocks: vec![Block::Search(SearchView {
            glyph: p.glyph("launcher-search"),
            query: search.query.clone().into(),
            placeholder: "Search apps".into(),
            empty: "No apps found".into(),
            filters: None,
            rows,
            pictures: false,
            on_scroll: p.on_local_with::<Launcher, i32>(move |_, local, rows| local.scroll(rows)),
            preview: None,
            fill: false,
            compact: false,
            list_height: None,
            status: None,
            list_width: None,
            beside: Vec::new(),
            focused: true,
            on_focus: None,
            table: None,
        })],
    })
}
