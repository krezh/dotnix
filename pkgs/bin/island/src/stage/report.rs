//! What `island ctl` hears back, whatever draws the islands.

use crate::{
    config::Frontend,
    control::{Report, Row, Shown},
    features::Features,
    wm::Snapshot,
};

pub fn status(frontend: Frontend, workspaces: Option<&Snapshot>, features: &Features) -> Report {
    let outputs: Vec<String> = workspaces
        .iter()
        .flat_map(|snapshot| snapshot.monitors.iter().map(|monitor| monitor.name.clone()))
        .collect();
    let on = if outputs.is_empty() {
        String::new()
    } else {
        format!(" · running on {}", outputs.join(", "))
    };
    let drawn = Row::new("Frontend", Shown::Text(frontend.name().to_owned()));
    Report {
        title: Some(format!("island {}{on}", env!("CARGO_PKG_VERSION"))),
        changed: false,
        rows: std::iter::once(drawn).chain(features.status()).collect(),
    }
}

/// `open` is false for a toggle that closed it.
pub fn opened(name: &str, view: Option<&str>, output: Option<String>, open: bool) -> Report {
    let output = output
        .map(|output| format!(" on {output}"))
        .unwrap_or_default();
    let shown = view.map(|view| format!(" on {view}")).unwrap_or_default();
    Report::did(Row::said(if open {
        format!("Opened {name}{shown}{output}")
    } else {
        format!("Closed {name}{output}")
    }))
}
