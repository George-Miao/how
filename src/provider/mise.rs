use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use super::{Confidence, Detection, DetectionContext, Provider};
use crate::util::{self, detect_paths, executable_name};

pub(super) struct Mise;
pub(super) static PROVIDER: Mise = Mise;
static ROOT: OnceLock<Option<PathBuf>> = OnceLock::new();

impl Provider for Mise {
    fn detect(&self, context: &DetectionContext<'_>) -> Option<Detection> {
        detect_paths(context, detect_path)
    }
}

fn detect_path(path: &Path) -> Option<Detection> {
    let root = root()?;
    if let Some(tool) = util::child_after(path, root, "installs") {
        return Some(Detection::path(
            "mise",
            Some(tool),
            Confidence::High,
            format!("target lives in mise data directory {}", root.display()),
        ));
    }
    util::executable_is_in(path, &root.join("shims")).then(|| {
        Detection::path(
            "mise",
            executable_name(path),
            Confidence::Medium,
            format!("executable is a shim in {}", root.join("shims").display()),
        )
    })
}

fn root() -> Option<&'static PathBuf> {
    ROOT.get_or_init(|| {
        util::env_path("MISE_DATA_DIR")
            .or_else(|| util::env_path("XDG_DATA_HOME").map(|path| path.join("mise")))
            .or_else(|| util::home_dir().map(|path| path.join(".local/share/mise")))
    })
    .as_ref()
}
