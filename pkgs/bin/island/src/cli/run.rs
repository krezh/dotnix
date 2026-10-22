use super::{completions::script, help, paint, program};

pub fn print(lines: &[paint::Line]) {
    println!("{}", paint::render(lines, paint::colour(false)));
}

pub fn print_error(lines: &[paint::Line]) {
    eprintln!("{}", paint::render(lines, paint::colour(true)));
}

/// `island --help`, or the help for `command`, one of `island`'s own.
pub fn print_help(command: Option<&str>) {
    let lines = match command.and_then(program::find) {
        Some(sub) => help::command(&sub),
        None => help::overview(),
    };
    print(&lines);
}

/// `island completions SHELL`; what the process exits with.
pub fn completions(args: Vec<String>) -> i32 {
    if args.iter().any(|arg| arg == "-h" || arg == "--help") {
        print_help(Some("completions"));
        return 0;
    }
    match script(&args) {
        Ok(script) => {
            print!("{script}");
            0
        }
        Err(mistake) => {
            print_error(&mistake.lines());
            2
        }
    }
}
