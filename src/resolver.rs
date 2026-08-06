use std::collections::HashSet;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use snafu::{OptionExt, ensure};

use crate::error::{CommandNotFoundSnafu, Error, NotExecutableSnafu, PathNotSetSnafu};

pub fn resolve(command: &OsStr, all: bool) -> Result<Vec<PathBuf>, Error> {
    let path = Path::new(command);
    if path.components().count() > 1 || path.is_absolute() {
        let resolved = explicit_candidates(path)
            .into_iter()
            .find(|candidate| is_executable(candidate));
        ensure!(
            resolved.is_some(),
            NotExecutableSnafu {
                path: path.to_path_buf(),
            }
        );
        return Ok(vec![resolved.expect("checked above")]);
    }

    let path_variable = std::env::var_os("PATH").context(PathNotSetSnafu)?;
    let mut matches = Vec::new();
    let mut seen = HashSet::new();
    for directory in std::env::split_paths(&path_variable) {
        for candidate in command_candidates(&directory, command) {
            if is_executable(&candidate) && seen.insert(candidate.clone()) {
                matches.push(candidate);
                if !all {
                    return Ok(matches);
                }
            }
        }
    }

    ensure!(
        !matches.is_empty(),
        CommandNotFoundSnafu {
            command: command.to_os_string()
        }
    );
    Ok(matches)
}

cfg_select! {
    windows => {
        fn command_candidates(directory: &Path, command: &OsStr) -> Vec<PathBuf> {
            use std::ffi::OsString;

            let path = Path::new(command);
            if path.extension().is_some() {
                return vec![directory.join(path)];
            }
            executable_extensions()
                .into_iter()
                .map(|extension| {
                    let mut name = OsString::from(command);
                    name.push(extension);
                    directory.join(name)
                })
                .collect()
        }

        fn explicit_candidates(path: &Path) -> Vec<PathBuf> {
            let Some(parent) = path.parent() else {
                return vec![path.to_path_buf()];
            };
            let Some(name) = path.file_name() else {
                return vec![path.to_path_buf()];
            };
            command_candidates(parent, name)
        }

        fn executable_extensions() -> Vec<std::ffi::OsString> {
            use std::ffi::OsString;

            std::env::var_os("PATHEXT")
                .map(|extensions| {
                    extensions
                        .to_string_lossy()
                        .split(';')
                        .filter(|extension| !extension.is_empty())
                        .map(OsString::from)
                        .collect()
                })
                .filter(|extensions: &Vec<OsString>| !extensions.is_empty())
                .unwrap_or_else(|| {
                    [".COM", ".EXE", ".BAT", ".CMD"]
                        .map(OsString::from)
                        .to_vec()
                })
        }

        pub(crate) fn is_executable(path: &Path) -> bool {
            path.is_file()
        }
    }
    unix => {
        fn command_candidates(directory: &Path, command: &OsStr) -> Vec<PathBuf> {
            vec![directory.join(command)]
        }

        fn explicit_candidates(path: &Path) -> Vec<PathBuf> {
            vec![path.to_path_buf()]
        }

        pub(crate) fn is_executable(path: &Path) -> bool {
            use std::os::unix::fs::PermissionsExt;

            std::fs::metadata(path)
                .map(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
                .unwrap_or(false)
        }
    }
    _ => {
        fn command_candidates(directory: &Path, command: &OsStr) -> Vec<PathBuf> {
            vec![directory.join(command)]
        }

        fn explicit_candidates(path: &Path) -> Vec<PathBuf> {
            vec![path.to_path_buf()]
        }

        pub(crate) fn is_executable(path: &Path) -> bool {
            path.is_file()
        }
    }
}
