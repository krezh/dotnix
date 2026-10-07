mod activation;
mod app;
mod build;
mod changelog;
mod cleanup;
mod config;
mod nix;
mod report;
mod repository;
mod state;
pub(crate) mod theme;
mod ui;

fn main() -> gtk::glib::ExitCode {
    app::run()
}
