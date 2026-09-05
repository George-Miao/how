use std::path::{Path, PathBuf};

use super::{Confidence, Detection, DetectionContext, Provider};
use crate::util::{self, detect_paths, executable_name};

pub(super) struct Go;
pub(super) static PROVIDER: Go = Go;

impl Provider for Go {
    fn detect(&self, context: &DetectionContext<'_>) -> Option<Detection> {
        let bin_dirs = configured_bin_dirs();
        detect_paths(context, |path| detect_path(path, &bin_dirs))
    }

    fn discover(&self, context: &DetectionContext<'_>) -> Option<Detection> {
        let bin_dirs = discover_bin_dirs(context.probe);
        detect_paths(context, |path| detect_path(path, &bin_dirs))
    }
}

fn detect_path(path: &Path, bin_dirs: &[PathBuf]) -> Option<Detection> {
    let directory = bin_dirs
        .iter()
        .find(|directory| util::executable_is_in(path, directory))?;
    Some(Detection::path(
        "Go",
        executable_name(path),
        Confidence::Medium,
        format!(
            "executable is in Go install directory {}",
            directory.display()
        ),
    ))
}

fn configured_bin_dirs() -> Vec<PathBuf> {
    if let Some(gobin) = util::env_path("GOBIN") {
        return vec![gobin];
    }
    if let Some(gopath) = std::env::var_os("GOPATH") {
        return util::path_list(&gopath)
            .into_iter()
            .map(|path| path.join("bin"))
            .collect();
    }
    util::home_dir()
        .map(|home| vec![home.join("go/bin")])
        .unwrap_or_default()
}

fn discover_bin_dirs(probe: &dyn crate::command_probe::CommandProbe) -> Vec<PathBuf> {
    if let Some(gobin) = util::command_path(probe, "go", &["env", "GOBIN"])
        && !gobin.as_os_str().is_empty()
    {
        return vec![gobin];
    }
    util::command_output(probe, "go", &["env", "GOPATH"])
        .map(|gopath| {
            util::path_list(gopath.as_ref())
                .into_iter()
                .map(|path| path.join("bin"))
                .collect()
        })
        .unwrap_or_default()
}
