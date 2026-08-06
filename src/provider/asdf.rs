use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use super::{Confidence, Detection, DetectionContext, Provider};
use crate::util::{self, detect_paths, executable_name};

pub(super) struct Asdf;
pub(super) static PROVIDER: Asdf = Asdf;
static ROOT: OnceLock<Option<PathBuf>> = OnceLock::new();

impl Provider for Asdf {
    fn detect(&self, context: &DetectionContext<'_>) -> Option<Detection> {
        detect_paths(context, detect_path)
    }
}

fn detect_path(path: &Path) -> Option<Detection> {
    let root = root()?;
    if let Some(tool) = util::child_after(path, root, "installs") {
        return Some(Detection::path(
            "asdf",
            Some(tool),
            Confidence::High,
            format!("target lives in asdf data directory {}", root.display()),
        ));
    }
    util::executable_is_in(path, &root.join("shims")).then(|| {
        Detection::path(
            "asdf",
            executable_name(path),
            Confidence::Medium,
            format!("executable is a shim in {}", root.join("shims").display()),
        )
    })
}

fn root() -> Option<&'static PathBuf> {
    ROOT.get_or_init(|| {
        util::env_path("ASDF_DATA_DIR").or_else(|| util::home_dir().map(|path| path.join(".asdf")))
    })
    .as_ref()
}
