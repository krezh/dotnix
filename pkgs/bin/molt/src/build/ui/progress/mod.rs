//! Live build progress: flake-input and work rows, their text, and the summary cards.

mod cards;
mod evaluation;
mod format;
mod work;

pub(crate) use cards::{DownloadsCard, EvaluationCard, ProgressCard, WarningsCard, card};
pub(crate) use evaluation::{
    INPUT_GROUPS, fail_open_fetches, input_rows, last_evaluation, remember_evaluation,
};
pub(crate) use format::{activity_name, clock};
pub(crate) use work::{WORK_GROUPS, WorkPlan, WorkSummary, download_bytes};
