mod activation;
mod app;
mod build;
mod cache;
mod check;
mod cleanup;
mod config;
mod home;
mod inputs;
mod nix;
mod report;
mod ui;
mod watchlist;

fn main() -> gtk::glib::ExitCode {
    // `check` is for scripts: it must not register a GApplication or touch the display.
    let mut arguments = std::env::args_os().skip(1);
    if arguments.next().is_some_and(|argument| argument == "check") {
        return check::run(arguments);
    }
    app::run()
}
