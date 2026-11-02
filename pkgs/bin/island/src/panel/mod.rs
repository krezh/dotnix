mod block;
mod frame;
mod model;
mod ui;
mod view;

pub use crate::components::{
    actions::{Action, Actions},
    badge::Mark,
    calendar::CalendarView,
    card::{Countdown, FrameView},
    chart::Series,
    code::{Change, Code, CodeLine, Motion as CodeMotion},
    dropdown::{Choice, Dropdown},
    input::{Edit, InputView},
    item::{Item, Lead, Trail},
    lines::Line,
    meter::{Meter, Shape as MeterShape},
    player::PlayerView,
    prompt::{PromptView, Reveal},
    route::{Route, Stop, StopState},
    search::{Preview, RowIcon, SearchRow, SearchView},
    slider::{Slider, SliderLabel},
    tabs::TabLabel,
    text::{Style as TextStyle, Text, Tone},
    transcript::{Piece, Transcript, Turn},
};

pub use frame::Frame;
pub use gpui_shared_string::SharedString;
pub use model::{
    ActionTile, Block, Button, ButtonRow, COLUMNS, ChartView, Handler, Header, HeaderLead,
    IconButton, PanelView, Passing, Place, Placed, Split, Views, Warning, on,
};
pub use ui::{IslandUi, Ui};

pub use block::PLACED_MARGIN;
