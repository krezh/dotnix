//! `island ctl`: checks a command, hands it to the running island, and shows its answer.

use super::{help, parse::parse, protocol, show, spec};
use crate::cli::{colour, print, print_error};

const JSON: &str = "--json";

/// What the process exits with.
pub fn run(args: Vec<String>) -> i32 {
    if args.iter().any(|arg| arg == "-h" || arg == "--help") {
        let lines = match args.first().and_then(|word| spec::find(word)) {
            Some((_, command)) => help::command(command),
            None => help::overview(),
        };
        print(&lines);
        return 0;
    }
    let json = args.iter().any(|arg| arg == JSON);
    let args: Vec<String> = args.into_iter().filter(|arg| arg != JSON).collect();
    if let Err(mistake) = parse(&args) {
        print_error(&mistake.lines());
        return 2;
    }
    match protocol::send(&args) {
        Ok(report) if json => {
            println!(
                "{}",
                serde_json::to_string_pretty(&report.json()).unwrap_or_default()
            );
            0
        }
        Ok(report) => {
            let mut lines = show::lines(&report, colour(false));
            if args.is_empty() {
                lines.push(crate::cli::Line::new());
                lines.push(help::more());
            }
            print(&lines);
            0
        }
        Err(mistake) => {
            print_error(&mistake.lines());
            1
        }
    }
}
