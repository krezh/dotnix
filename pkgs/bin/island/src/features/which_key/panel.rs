use super::{
    commands::Picked,
    feature::WhichKey,
    menu::{self, Level},
    settings::Entry,
    state::Local,
};
use crate::{
    components::card::{Look, Motion},
    features::PanelCx,
    panel::{Block, FrameView, Header, Item, Lead, Mark, PanelView, SharedString, Trail},
    theme::{OVERLAY1, app_color},
    units::capitalise,
};

pub(super) fn panel(which_key: &WhichKey, p: &PanelCx<'_>) -> Option<PanelView> {
    let local = p.local::<Local>()?;
    let name = local.menu();
    let Some(Level { path, entries }) = menu::level(&which_key.settings, name, &local.keys) else {
        return Some(PanelView {
            header: Header::Path(vec![capitalise(name).into()]),
            blocks: vec![Block::note(format!("No menu named {name}"))],
            scroll: false,
        });
    };
    let at = path.join("/");
    let rows = entries.iter().map(|entry| row(entry, &at, p)).collect();
    let mut hints = String::from("Esc close");
    if !local.keys.is_empty() {
        hints.push_str(" · Backspace back");
    }
    Some(PanelView {
        header: Header::Path(path.into_iter().map(SharedString::from).collect()),
        blocks: vec![
            Block::Grid(menu::columns(entries.len()), rows),
            Block::caption(hints, OVERLAY1),
        ],
        scroll: false,
    })
}

fn row(entry: &Entry, at: &str, p: &PanelCx<'_>) -> Block {
    let on_click = if entry.submenu.is_empty() {
        let picked = entry.clone();
        p.on_island::<WhichKey>(move |which_key, ui| {
            if let Picked::Run = which_key.pick(&picked) {
                ui.collapse();
            }
        })
    } else {
        let key = entry.key.clone();
        p.on_local::<WhichKey>(move |_, local| local.enter(&key))
    };
    let trail = if entry.submenu.is_empty() {
        Vec::new()
    } else {
        vec![Trail::Faint(format!("{} ›", entry.submenu.len()).into())]
    };
    let item = Item {
        id: format!("which-key-{at}-{}", entry.key).into(),
        lead: Some(Lead {
            mark: Some(Mark::Key(entry.key.clone().into(), app_color(&entry.desc))),
            ..Default::default()
        }),
        title: entry.desc.clone().into(),
        trail,
        ..Default::default()
    };
    Block::Frame(
        FrameView {
            id: format!("which-key-row-{at}-{}", entry.key).into(),
            look: Look::Plain,
            on_click: Some(on_click),
            on_dismiss: None,
            countdown: None,
            selected: false,
            motion: Motion::Still,
            shown_height: None,
            min_height: None,
        },
        vec![Block::Item(item, Vec::new())],
    )
}
