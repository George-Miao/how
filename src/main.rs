mod cli;
mod command_probe;
mod detect;
mod error;
mod provider;
mod resolver;
mod shell;
mod util;

use std::process::ExitCode;

fn main() -> ExitCode {
    match cli::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("how: {error}");
            ExitCode::from(1)
        }
    }
}
