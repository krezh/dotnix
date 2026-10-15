mod backend;
mod blur;
mod cli;
mod config;
mod ui;

use std::process::ExitCode;

fn detach(no_fork: bool) -> Result<bool, String> {
    unsafe {
        if libc::signal(libc::SIGHUP, libc::SIG_IGN) == libc::SIG_ERR {
            return Err(format!(
                "cannot ignore SIGHUP: {}",
                std::io::Error::last_os_error()
            ));
        }
        if no_fork {
            return Ok(true);
        }
        // Fork before GTK or worker threads exist; the orphan survives its launching terminal.
        match libc::fork() {
            -1 => {
                return Err(format!(
                    "cannot detach: {}",
                    std::io::Error::last_os_error()
                ));
            }
            0 => {}
            _ => return Ok(false),
        }
        if libc::setsid() == -1 {
            return Err(format!(
                "cannot create session: {}",
                std::io::Error::last_os_error()
            ));
        }
        match libc::fork() {
            -1 => {
                return Err(format!(
                    "cannot detach: {}",
                    std::io::Error::last_os_error()
                ));
            }
            0 => {}
            _ => libc::_exit(0),
        }
    }
    Ok(true)
}

fn run() -> Result<(), String> {
    let options = match cli::parse(std::env::args().skip(1))? {
        cli::Command::Help => {
            println!("{}", cli::HELP);
            return Ok(());
        }
        cli::Command::Run(options) => options,
    };
    if !std::env::var("HYPRLAND_INSTANCE_SIGNATURE").is_ok_and(|value| !value.is_empty()) {
        return Err("Dusk requires a Hyprland session".into());
    }
    if !detach(options.no_fork)? {
        return Ok(());
    }
    let session = backend::Session::connect(&options)?;
    let (outcome, mut session) = ui::run(options.clone(), session)?;
    if matches!(outcome, ui::Outcome::Complete) {
        session.finish(&options)?;
    }
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("dusk: {error}");
            ExitCode::FAILURE
        }
    }
}
