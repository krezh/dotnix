use super::{Session, local, local::Exit, state::State};
use crate::{
    bar_widget::BarWidget,
    features::{GlyphSpec, IslandInfo, PanelCx, Surface, WidgetCx},
    panel::{ActionTile, Block, Header, PanelView},
    sys,
    theme::TEXT,
    units::{Label, next_change, span},
};

/// "Up 2 d 3 h", "Up 3 h 12 min", "Up 12 min".
fn uptime(p: &PanelCx<'_>) -> Option<String> {
    let up = sys::uptime()?;
    p.redraw_in(next_change(Label::Span, up));
    Some(format!("Up {}", span(up.as_secs())))
}

impl Surface for Session {
    const GLYPHS: &'static [GlyphSpec] = &[
        ("session-lock", &["lock"]),
        ("session-logout", &["log-out"]),
        ("session-suspend", &["moon"]),
        ("session-hibernate", &["snowflake"]),
        ("session-reboot", &["rotate-ccw"]),
        ("session-shutdown", &["power"]),
    ];

    const PANEL: Option<&'static str> = Some("session");

    type Local = Exit;

    fn sync_local(state: &State, exit: &mut Exit, island: &IslandInfo<'_>, animate: bool) {
        local::sync(state, exit, island, animate);
    }

    fn widget(&self, _: &str, _: &State, w: &mut WidgetCx<'_>) -> Option<BarWidget> {
        Some(
            w.base()
                .tip("Session")
                .glyph(w.glyph("session-shutdown"), TEXT)
                .on_click(w.toggle(Self::panel_handle())),
        )
    }

    fn panel<'a>(&self, state: &State, p: &PanelCx<'a>) -> Option<PanelView> {
        let armed = state.armed;
        let tiles = self
            .settings
            .actions
            .iter()
            .map(|&action| ActionTile {
                glyph: p.glyph(&format!("session-{}", action.word())),
                label: action.label().into(),
                state: None,
                color: action.color(),
                armed: armed == Some(action),
                on_click: p.on_island::<Self>(move |session, ui| {
                    if session.press(action) && !action.ends_session() {
                        ui.collapse();
                    }
                }),
            })
            .collect();
        let note = match armed {
            Some(action) => format!("Click {} again to confirm", action.label()),
            None => uptime(p).unwrap_or_default(),
        };
        Some(PanelView {
            scroll: false,
            header: Header::None,
            blocks: vec![Block::Actions(tiles), Block::note(note)],
        })
    }

    fn popup<'a>(&self, state: &State, p: &PanelCx<'a>) -> Option<PanelView> {
        self.exit_view(state.leaving.as_ref()?, p)
    }
}
