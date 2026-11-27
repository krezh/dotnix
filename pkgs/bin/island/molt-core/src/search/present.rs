//! The Search screen as it is shown: every word on it, for any front end to lay out.

use serde_json::Value;

use crate::search::flow::{Local, Search, local_key};
use crate::search::model::{PAGE_SIZE, RESULT_WINDOW, SearchResultItem, SearchScope};

pub const SCOPES: [&str; 4] = ["All", "Packages", "NixOS", "Home Manager"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Package,
    NixOsOption,
    HomeManagerOption,
}

pub struct SearchView {
    pub title: &'static str,
    pub subtitle: &'static str,
    pub note: &'static str,
    pub placeholder: &'static str,
    pub query: String,
    pub scopes: Vec<String>,
    pub scope: usize,
    pub status: String,
    pub loading: bool,
    pub failed: bool,
    pub notice: Option<String>,
    pub rows: Vec<Row>,
    pub detail: Option<Detail>,
    pub nothing: &'static str,
    pub previous: bool,
    pub next: bool,
}

pub struct Row {
    pub name: String,
    /// Where the query's words are in the name, as byte ranges.
    pub marks: Vec<(usize, usize)>,
    pub detail: String,
    pub kind: Kind,
    pub selected: bool,
}

pub struct Section {
    pub caption: &'static str,
    pub text: String,
    /// A value to show as code rather than words.
    pub code: bool,
    pub muted: bool,
}

pub struct Detail {
    pub kind: Kind,
    pub category: &'static str,
    pub name: String,
    pub copy_name: (&'static str, String),
    pub snippet: String,
    pub sections: Vec<Section>,
    pub links: Vec<(&'static str, String)>,
    /// What checking against the user's flake says, when there is a configuration to check.
    pub local: Option<LocalView>,
}

pub enum LocalView {
    Idle {
        note: &'static str,
        action: &'static str,
    },
    Running(&'static str),
    Done {
        summary: &'static str,
        value: String,
        attribute: String,
        action: &'static str,
    },
    Failed {
        error: String,
        action: &'static str,
    },
}

fn compact_count(count: usize) -> String {
    if count < 1000 {
        count.to_string()
    } else {
        format!("{:.1}k", count as f64 / 1000.0)
    }
}

fn identity(item: &SearchResultItem) -> (&str, &'static str, Kind) {
    match item {
        SearchResultItem::Package(hit) => (&hit.attr_name, "Package", Kind::Package),
        SearchResultItem::NixOsOption(hit) => (&hit.name, "NixOS option", Kind::NixOsOption),
        SearchResultItem::HomeManagerOption(hit) => {
            (&hit.name, "Home Manager option", Kind::HomeManagerOption)
        }
    }
}

/// Every place a query word appears in `name`, ignoring ASCII case.
pub fn marks(name: &str, query: &str) -> Vec<(usize, usize)> {
    let lower = name.to_ascii_lowercase();
    let mut marked = vec![false; name.len()];
    for word in query
        .split(|character: char| character.is_whitespace() || character == '.')
        .filter(|word| !word.is_empty())
    {
        let word = word.to_ascii_lowercase();
        let mut from = 0;
        while let Some(found) = lower[from..].find(&word) {
            let start = from + found;
            marked[start..start + word.len()].fill(true);
            from = start + word.len();
        }
    }
    let mut ranges = Vec::new();
    let mut start = None;
    for (index, on) in marked.iter().enumerate() {
        match (on, start) {
            (true, None) => start = Some(index),
            (false, Some(from)) => {
                ranges.push((from, index));
                start = None;
            }
            _ => {}
        }
    }
    if let Some(from) = start {
        ranges.push((from, name.len()));
    }
    ranges
}

/// Words from a description, which the catalog gives as Markdown or as rendered HTML.
pub fn plain(text: &str) -> String {
    let Some(html) = text
        .trim()
        .strip_prefix("<rendered-html>")
        .and_then(|rest| rest.strip_suffix("</rendered-html>"))
    else {
        return text.trim().to_owned();
    };
    let mut words = String::new();
    let mut inside = false;
    for character in html.chars() {
        match character {
            '<' => inside = true,
            '>' => inside = false,
            _ if !inside => words.push(character),
            _ => {}
        }
    }
    words
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&amp;", "&")
        .trim()
        .to_owned()
}

fn text(caption: &'static str, value: Option<&str>) -> Option<Section> {
    value
        .filter(|value| !value.is_empty())
        .map(|value| Section {
            caption,
            text: value.to_owned(),
            code: false,
            muted: false,
        })
}

fn value(caption: &'static str, value: &Value) -> Section {
    let (text, code, muted) = match value {
        Value::Null => ("Not provided".to_owned(), false, true),
        Value::String(words) if words.trim().starts_with("<rendered-html>") => {
            (plain(words), false, false)
        }
        Value::String(words) => (words.clone(), true, false),
        other => (other.to_string(), true, false),
    };
    Section {
        caption,
        text,
        code,
        muted,
    }
}

fn detail(screen: &Search, item: &SearchResultItem, revision: Option<&str>) -> Detail {
    let (name, category, kind) = identity(item);
    let mut sections = Vec::new();
    match item {
        SearchResultItem::Package(hit) => {
            sections.extend(text("Version", hit.version.as_deref()));
            sections.extend(text(
                "Description",
                hit.description.as_deref().map(plain).as_deref(),
            ));
            if !hit.licenses.is_empty() {
                sections.extend(text("Licenses", Some(&hit.licenses.join(", "))));
            }
            if !hit.programs.is_empty() {
                sections.extend(text("Provided programs", Some(&hit.programs.join(", "))));
            }
            sections.extend(text("Declaration", hit.position.as_deref()));
        }
        SearchResultItem::NixOsOption(hit) | SearchResultItem::HomeManagerOption(hit) => {
            sections.extend(text(
                "Description",
                hit.description.as_deref().map(plain).as_deref(),
            ));
            sections.extend(text("Type", hit.option_type.as_deref()));
            sections.push(value("Default", &hit.default_value));
            sections.push(value("Example", &hit.example));
            sections.extend(text("Source", hit.source.as_deref()));
        }
    }
    let mut links = Vec::new();
    if let Some(url) = item.source_url(revision) {
        links.push(("View source", url));
    }
    if let SearchResultItem::Package(hit) = item
        && let Some(url) = hit.homepage_url()
    {
        links.push(("Open homepage", url.to_owned()));
    }
    let local = screen
        .config
        .as_ref()
        .map(|_| match screen.local.get(&local_key(item)) {
            None => LocalView::Idle {
                note: "Evaluate this against your flake's locked nixpkgs and configuration",
                action: "Check locally",
            },
            Some(Local::Running) => LocalView::Running("Evaluating with nix eval…"),
            Some(Local::Done(Ok(found))) => LocalView::Done {
                summary: match (item, found.set) {
                    (SearchResultItem::Package(_), _) => "Version in your nixpkgs",
                    (_, Some(true)) => "Set in your configuration",
                    (_, Some(false)) => "Not set; the module default applies",
                    (_, None) => "Value in your configuration",
                },
                value: found.value.clone(),
                attribute: found.attribute.clone(),
                action: "Check again",
            },
            Some(Local::Done(Err(error))) => LocalView::Failed {
                error: error.clone(),
                action: "Retry",
            },
        });
    Detail {
        kind,
        category,
        name: name.to_owned(),
        copy_name: (
            if kind == Kind::Package {
                "Copy attribute"
            } else {
                "Copy option"
            },
            name.to_owned(),
        ),
        snippet: item.snippet(),
        sections,
        links,
        local,
    }
}

pub fn view(screen: &Search) -> SearchView {
    let scopes = SCOPES
        .iter()
        .zip(screen.queue.counts)
        .map(|(name, count)| {
            count.map_or_else(
                || (*name).to_owned(),
                |count| format!("{name} · {}", compact_count(count)),
            )
        })
        .collect();
    let scope = SearchScope::ALL
        .iter()
        .position(|scope| *scope == screen.scope)
        .unwrap_or(0);
    let mut shown = SearchView {
        title: "Search",
        subtitle: "nixos-unstable · search.nixos.org",
        note: "Catalog results may differ from your locked inputs; check a result locally to see yours",
        placeholder: "Search packages and options · p: o: hm: to pick a category",
        query: screen.query.clone(),
        scopes,
        scope,
        status: if screen.loading {
            "Searching…".to_owned()
        } else {
            "Search packages and configuration options".to_owned()
        },
        loading: screen.loading,
        failed: false,
        notice: None,
        rows: Vec::new(),
        detail: None,
        nothing: "Select a result to inspect",
        previous: false,
        next: false,
    };
    if let Some(error) = &screen.error {
        shown.status = "Search failed".to_owned();
        shown.notice = Some(error.clone());
        shown.failed = true;
        return shown;
    }
    let Some((offset, results)) = &screen.shown else {
        return shown;
    };
    if results.total == 0 {
        shown.status = format!("No results for ‘{}’", screen.query);
        return shown;
    }
    shown.status = format!(
        "Showing {}–{} of {} results · {} ms",
        offset + 1,
        offset + results.items.len(),
        results.total,
        results.elapsed.as_millis()
    );
    shown.previous = *offset > 0;
    shown.next = offset + PAGE_SIZE < results.total.min(RESULT_WINDOW);
    if results.total > RESULT_WINDOW && offset + PAGE_SIZE >= RESULT_WINDOW {
        shown.notice = Some("Refine your query to see more than 10,000 results".to_owned());
    }
    shown.rows = results
        .items
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let (name, category, kind) = identity(item);
            let detail = match item {
                SearchResultItem::Package(hit) => {
                    hit.version.as_deref().unwrap_or(category).to_owned()
                }
                SearchResultItem::NixOsOption(hit) | SearchResultItem::HomeManagerOption(hit) => {
                    hit.option_type.as_deref().map_or_else(
                        || category.to_owned(),
                        |kind| format!("{category} · {kind}"),
                    )
                }
            };
            Row {
                name: name.to_owned(),
                marks: marks(name, &screen.query),
                detail,
                kind,
                selected: index == screen.selected,
            }
        })
        .collect();
    shown.detail = screen
        .current()
        .map(|item| detail(screen, item, results.revision.as_deref()));
    shown
}
