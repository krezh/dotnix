mod complete;
mod completions;
mod help;
mod mistake;
mod options;
mod paint;
mod program;
mod run;

pub use complete::print as print_completions;
pub use help::{heading, row, title, under, usage};
pub use mistake::{Mistake, either, listed, not_running};
pub use options::parse as parse_options;
#[cfg(test)]
pub use paint::render;
pub use paint::{Line, Tone, colour};
pub use program::{Client, ISLAND, LOGS, Opt, commands};
pub use run::{completions, print, print_error, print_help};
