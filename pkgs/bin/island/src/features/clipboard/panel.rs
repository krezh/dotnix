use std::path::Path;

use super::{
    class::{self, Class},
    entry::Entry,
    filter::FILTERS,
    picker::Picker,
    runtime::{Clipboard, Request},
    state::State,
};
use crate::{
    components::list::ROWS,
    features::PanelCx,
    panel::{
        Block, Code, CodeLine, CodeMotion, FileRow, Filters, Header, IconButton, PanelView,
        Chip, Preview, RowIcon, SearchPreview, SearchRow, SearchView, TabLabel,
    },
    theme::{ACCENT, GREEN, LAVENDER, PEACH, PINK, RED, SAPPHIRE, YELLOW},
};

const PICTURES: &[&str] = &[
    "avif", "bmp", "gif", "heic", "jpeg", "jpg", "jxl", "png", "svg", "tif", "tiff", "webp",
];

fn color(class: Class) -> u32 {
    match class {
        Class::Text => LAVENDER,
        Class::Link => SAPPHIRE,
        Class::Code => GREEN,
        Class::Color => PINK,
        Class::Picture => PEACH,
        Class::Files => YELLOW,
    }
}

fn glyph_key(class: Class) -> &'static str {
    match class {
        Class::Text => "clipboard-text",
        Class::Link => "clipboard-link",
        Class::Code => "clipboard-code",
        Class::Color => "clipboard-color",
        Class::Picture => "clipboard-image",
        Class::Files => "clipboard-files",
    }
}

fn label(class: Class) -> &'static str {
    match class {
        Class::Text => "Text",
        Class::Link => "Link",
        Class::Code => "Code",
        Class::Color => "Color",
        Class::Picture => "Picture",
        Class::Files => "Files",
    }
}

fn icon(entry: &Entry, p: &PanelCx<'_>) -> RowIcon {
    match (entry.class, &entry.preview, class::color(&entry.text)) {
        (Class::Picture, Some(picture), _) => RowIcon::Picture(picture.clone()),
        (Class::Color, _, Some(rgb)) => RowIcon::Swatch(rgb),
        (class, _, _) => p
            .glyph(glyph_key(class))
            .map_or(RowIcon::None, |path| RowIcon::Tinted(path, color(class))),
    }
}

pub(super) fn panel(state: &State, p: &PanelCx<'_>) -> Option<PanelView> {
    let picker = p.local::<Picker>()?;
    let query = &picker.selection.query;
    let matching = state.matching(query, picker.filter);
    let layout = matching.layout;
    let count = layout.count;
    let selected = picker.selection.selected.min(count.saturating_sub(1));
    let top = picker.selection.top.min(count.saturating_sub(ROWS));
    let pin = p.glyph("clipboard-pin");
    let rows = matching
        .entries
        .iter()
        .enumerate()
        .skip(top)
        .take(ROWS)
        .map(|(position, entry)| {
            p.redraw_in(entry.next_change());
            let id = entry.id;
            SearchRow {
                name: entry.title().into(),
                detail: entry.detail().into(),
                icon: icon(entry, p),
                mono: entry.class == Class::Code,
                section: layout.section(position).map(Into::into),
                mark: pin.clone().filter(|_| entry.pinned).map(|path| (path, YELLOW)),
                trail: Vec::new(),
                selected: position == selected,
                on_click: p.on_island::<Clipboard>(move |clipboard, ui| {
                    clipboard.request(Request::Select { id, paste: true });
                    ui.collapse();
                }),
                on_hover: p
                    .on_local::<Clipboard>(move |_, picker| picker.select(position, layout)),
            }
        })
        .collect();
    let counts = state.counts(query);
    let filters = Filters {
        tabs: FILTERS
            .iter()
            .zip(counts)
            .map(|(filter, count)| TabLabel {
                label: filter.label().into(),
                glyph: None,
                count: Some(count.to_string().into()),
            })
            .collect(),
        selected: FILTERS
            .iter()
            .position(|filter| *filter == picker.filter)
            .unwrap_or(0),
        on_select: p.on_local_with::<Clipboard, usize>(move |_, picker, index| {
            picker.filter(FILTERS[index.min(FILTERS.len() - 1)]);
        }),
    };
    let preview = match matching.entries.get(selected) {
        Some(entry) => {
            p.redraw_in(entry.next_change());
            preview(entry, p)
        }
        None => SearchPreview {
            tag: None,
            detail: "".into(),
            mark: None,
            content: Preview::Empty,
            buttons: Vec::new(),
            end: Vec::new(),
        },
    };
    let empty = match picker.filter.empty() {
        Some(words) => words.to_owned(),
        None => state
            .error
            .clone()
            .unwrap_or_else(|| "Nothing copied yet".to_owned()),
    };
    Some(PanelView {
        scroll: false,
        header: Header::None,
        blocks: vec![Block::Search(SearchView {
            glyph: p.glyph("clipboard-search"),
            query: query.clone().into(),
            placeholder: "Search the clipboard".into(),
            empty: empty.into(),
            filters: Some(filters),
            rows,
            pictures: false,
            on_scroll: p.on_local_with::<Clipboard, i32>(move |_, picker, rows| {
                picker.scroll(rows, layout);
            }),
            preview: Some(preview),
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

fn preview(entry: &Entry, p: &PanelCx<'_>) -> SearchPreview {
    let id = entry.id;
    let pinned = entry.pinned;
    let content = match entry.class {
        Class::Text => Preview::Text(entry.text.clone().into()),
        Class::Code => Preview::Code(Code {
            heading: None,
            lines: entry
                .text
                .lines()
                .zip(1..)
                .map(|(line, number)| CodeLine {
                    number: Some(number),
                    text: line.into(),
                    color: None,
                })
                .collect(),
            wrap: false,
            copy: None,
            motion: CodeMotion::Still,
            scroll: None,
        }),
        Class::Link => Preview::Link {
            glyph: p.glyph("clipboard-site"),
            color: SAPPHIRE,
            site: class::site(&entry.text).unwrap_or_default().into(),
            address: entry.text.trim().into(),
        },
        Class::Color => match class::color(&entry.text) {
            Some(rgb) => {
                let (hue, saturation, lightness) = class::hsl(rgb);
                let [_, r, g, b] = rgb.to_be_bytes();
                Preview::Swatch {
                    color: rgb,
                    values: vec![
                        ("Hex".into(), format!("#{rgb:06x}").into()),
                        ("RGB".into(), format!("{r}, {g}, {b}").into()),
                        (
                            "HSL".into(),
                            format!("{hue}°, {saturation}%, {lightness}%").into(),
                        ),
                    ],
                }
            }
            None => Preview::Text(entry.text.clone().into()),
        },
        Class::Picture => entry
            .preview
            .clone()
            .map_or(Preview::Empty, Preview::Picture),
        Class::Files => Preview::Files(entry.text.lines().map(|path| file_row(path, p)).collect()),
    };
    let button = |glyph: &str, label: &str, color: Option<u32>, active: bool, request: Request| {
        IconButton {
            glyph: p.glyph(glyph),
            color,
            label: label.into(),
            active,
            badge: None,
            enabled: true,
            busy: false,
            on_click: p.on_island::<Clipboard>(move |clipboard, ui| {
                let closes = matches!(request, Request::Select { .. });
                clipboard.request(request.clone());
                if closes {
                    ui.collapse();
                }
            }),
        }
    };
    SearchPreview {
        tag: Some(Chip {
            glyph: p.glyph(glyph_key(entry.class)),
            label: label(entry.class).into(),
            color: color(entry.class),
            busy: false,
            wide: false,
        }),
        detail: entry.about().into(),
        mark: p
            .glyph("clipboard-pin")
            .filter(|_| pinned)
            .map(|path| (path, YELLOW)),
        content,
        buttons: vec![
            button(
                "clipboard-paste",
                "Paste",
                Some(ACCENT),
                true,
                Request::Select { id, paste: true },
            ),
            button(
                "clipboard-copy",
                "Copy",
                None,
                false,
                Request::Select { id, paste: false },
            ),
            button(
                "clipboard-pin",
                if pinned { "Unpin" } else { "Pin" },
                pinned.then_some(YELLOW),
                false,
                Request::Pin {
                    id,
                    pinned: !pinned,
                },
            ),
        ],
        end: vec![button(
            "clipboard-forget",
            "Forget",
            Some(RED),
            false,
            Request::Delete(id),
        )],
    }
}

fn file_row(path: &str, p: &PanelCx<'_>) -> FileRow {
    let path = Path::new(path);
    let picture = path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| PICTURES.contains(&extension.to_ascii_lowercase().as_str()));
    let class = if picture { Class::Picture } else { Class::Text };
    let home = std::env::var_os("HOME").filter(|home| !home.is_empty());
    let place = path.parent().map_or(String::new(), |parent| {
        match home.as_deref().and_then(|home| parent.strip_prefix(home).ok()) {
            Some(within) if within.as_os_str().is_empty() => "~".to_owned(),
            Some(within) => format!("~/{}", within.display()),
            None => parent.display().to_string(),
        }
    });
    FileRow {
        icon: p
            .glyph(glyph_key(class))
            .map_or(RowIcon::None, |glyph| RowIcon::Tinted(glyph, color(class))),
        name: path
            .file_name()
            .map_or_else(|| path.display().to_string(), |name| {
                name.to_string_lossy().into_owned()
            })
            .into(),
        place: place.into(),
    }
}
