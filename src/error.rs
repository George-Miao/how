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
}
