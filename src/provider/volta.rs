use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use super::{Confidence, Detection, DetectionContext, Provider};
use crate::util::{self, detect_paths, executable_name};

pub(super) struct Volta;
pub(super) static PROVIDER: Volta = Volta;
static ROOT: OnceLock<Option<PathBuf>> = OnceLock::new();

impl Provider for Volta {
    fn detect(&self, context: &DetectionContext<'_>) -> Option<Detection> {
        detect_paths(context, detect_path)
    }
}

fn detect_path(path: &Path) -> Option<Detection> {
    let root = root()?;
    if let Some(package) = util::child_after(path, &root.join("tools/image"), "packages") {
        return Some(Detection::path(
            "Volta",
            Some(package),
            Confidence::High,
            format!("target lives under Volta home {}", root.display()),
        ));
    }
    util::executable_is_in(path, &root.join("bin")).then(|| {
        Detection::path(
            "Volta",
            executable_name(path),
            Confidence::Medium,
            format!("executable is a shim under Volta home {}", root.display()),
        )
    })
}

fn root() -> Option<&'static PathBuf> {
    ROOT.get_or_init(|| {
        util::env_path("VOLTA_HOME").or_else(|| util::home_dir().map(|path| path.join(".volta")))
    })
    .as_ref()
}
