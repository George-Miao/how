use std::ffi::OsString;
use std::path::PathBuf;

use snafu::Snafu;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum Error {
    #[snafu(display("PATH is not set"))]
    PathNotSet,

    #[snafu(display("{} is not an executable file", path.display()))]
    NotExecutable { path: PathBuf },

    #[snafu(display("command {command:?} was not found in PATH"))]
    CommandNotFound { command: OsString },

    #[snafu(display(
        "unsupported shell {shell:?}; supported shells: bash, zsh, fish, nu, pwsh, powershell, \
         tcsh, csh"
    ))]
    UnsupportedShell { shell: OsString },

    #[snafu(display("shell path {} is not an executable file", path.display()))]
    InvalidShellPath { path: PathBuf },

    #[snafu(display("shell {shell:?} could not be executed"))]
    ShellUnavailable { shell: OsString },
}
