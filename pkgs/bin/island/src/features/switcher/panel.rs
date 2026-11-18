use super::{
    runtime::Switcher,
    state::{Local, Previews},
};
use crate::{
    features::PanelCx,
    panel::{Block, GalleryCard, GalleryView, Header, Mark, PanelView},
    theme::{CRITICAL, app_color},
};

const KEYS: [(&str, &str); 5] = [
    ("Tab", "next"),
    ("Shift+Tab", "back"),
    ("§", "same app"),
    ("Q", "close"),
    ("Esc", "cancel"),
];

pub(super) fn panel(previews: &Previews, p: &PanelCx<'_>) -> Option<PanelView> {
    let local = p.local::<Local>()?;
    let cards = local
        .cards()
        .into_iter()
        .map(|(slot, place, staying)| {
            let window = &slot.item;
            let app = local.app(&window.class);
            let name = local.name(window);
            let where_ = match local.focused_monitor() {
                Some(focused) if focused != window.monitor => {
                    format!("{} · {}", window.place.label(), window.monitor)
                }
                _ => window.place.label(),
            };
            let address = window.address.clone();
            GalleryCard {
                key: slot.key.clone().into(),
                picture: previews
                    .shots
                    .of(&window.id, &window.class, &window.title)
                    .cloned(),
                mark: match app.icon {
                    Some(path) => match previews.icons.get(&path) {
                        Some(icon) => Mark::Picture(icon.clone(), false),
                        None => Mark::Image(path),
                    },
                    None => Mark::Letter(name.clone().into(), app_color(&window.class)),
                },
                lines: local.name_lines(&name),
                name: name.into(),
                detail: where_.into(),
                flag: window.urgent.then_some(CRITICAL),
                slot: place,
                presence: slot.presence.value(),
                on_click: staying.then(|| {
                    p.on_island::<Switcher>(move |switcher, ui| {
                        switcher.focus(&address);
                        ui.collapse();
                    })
                }),
                on_hover: staying
                    .then(|| p.on_local::<Switcher>(move |_, local| local.select(place))),
            }
        })
        .collect();
    let picked = local.picked();
    let caption = picked
        .map(|window| {
            if window.title.is_empty() {
                local.name(window)
            } else {
                window.title.clone()
            }
        })
        .unwrap_or_default();
    let places = local.shown_count();
    let gallery = Block::Gallery(GalleryView {
        cards,
        places,
        picked: picked.is_some().then_some(local.selected),
        first_row: local.first_row,
        caption_lines: local.caption_lines(&caption),
        caption: caption.into(),
        empty: "No open windows".into(),
        on_rows: p.on_local_with::<Switcher, i32>(|_, local, rows| local.scroll(rows)),
        keys: if places > 0 {
            KEYS.iter()
                .map(|(key, does)| ((*key).into(), (*does).into()))
                .collect()
        } else {
            Vec::new()
        },
    });
    Some(PanelView {
        scroll: false,
        header: Header::None,
        blocks: vec![gallery],
    })
}
