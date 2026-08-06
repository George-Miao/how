use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use super::{Confidence, Detection, DetectionContext, Provider};
use crate::util::{self, detect_paths, executable_name};

pub(super) struct Go;
pub(super) static PROVIDER: Go = Go;
static BIN_DIRS: OnceLock<Vec<PathBuf>> = OnceLock::new();

impl Provider for Go {
    fn detect(&self, context: &DetectionContext<'_>) -> Option<Detection> {
        detect_paths(context, detect_path)
    }
}

fn detect_path(path: &Path) -> Option<Detection> {
    let directory = bin_dirs()
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

fn bin_dirs() -> &'static [PathBuf] {
    BIN_DIRS.get_or_init(discover_bin_dirs)
}

fn discover_bin_dirs() -> Vec<PathBuf> {
    if let Some(gobin) = util::env_path("GOBIN") {
        return vec![gobin];
    }
    if let Some(gopath) = std::env::var_os("GOPATH") {
        return util::path_list(&gopath)
            .into_iter()
            .map(|path| path.join("bin"))
            .collect();
    }
    if let Some(gobin) = util::command_path("go", &["env", "GOBIN"])
        && !gobin.as_os_str().is_empty()
    {
        return vec![gobin];
    }
    if let Some(gopath) = util::command_output("go", &["env", "GOPATH"]) {
        return util::path_list(gopath.as_ref())
            .into_iter()
            .map(|path| path.join("bin"))
            .collect();
    }
    util::home_dir()
        .map(|home| vec![home.join("go/bin")])
        .unwrap_or_default()
}
