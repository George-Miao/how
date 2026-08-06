use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use super::{Confidence, Detection, DetectionContext, Provider};
use crate::util::{self, detect_paths, executable_name};

pub(super) struct Bun;
pub(super) static PROVIDER: Bun = Bun;
static ROOT: OnceLock<Option<PathBuf>> = OnceLock::new();

impl Provider for Bun {
    fn detect(&self, context: &DetectionContext<'_>) -> Option<Detection> {
        detect_paths(context, detect_path)
    }
}

fn detect_path(path: &Path) -> Option<Detection> {
    let bin = root()?.join("bin");
    util::executable_is_in(path, &bin).then(|| {
        Detection::path(
            "Bun",
            executable_name(path),
            Confidence::Medium,
            format!("executable is under Bun install root {}", bin.display()),
        )
    })
}

fn root() -> Option<&'static PathBuf> {
    ROOT.get_or_init(|| {
        util::env_path("BUN_INSTALL").or_else(|| util::home_dir().map(|home| home.join(".bun")))
    })
    .as_ref()
}
