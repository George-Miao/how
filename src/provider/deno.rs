use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use super::{Confidence, Detection, DetectionContext, Provider};
use crate::util::{self, detect_paths, executable_name};

pub(super) struct Deno;
pub(super) static PROVIDER: Deno = Deno;
static ROOT: OnceLock<Option<PathBuf>> = OnceLock::new();

impl Provider for Deno {
    fn detect(&self, context: &DetectionContext<'_>) -> Option<Detection> {
        detect_paths(context, detect_path)
    }
}

fn detect_path(path: &Path) -> Option<Detection> {
    let bin = root()?.join("bin");
    util::executable_is_in(path, &bin).then(|| {
        Detection::path(
            "Deno",
            executable_name(path),
            Confidence::Medium,
            format!("executable is under Deno install root {}", bin.display()),
        )
    })
}

fn root() -> Option<&'static PathBuf> {
    ROOT.get_or_init(|| {
        util::env_path("DENO_INSTALL_ROOT")
            .or_else(|| util::home_dir().map(|home| home.join(".deno")))
    })
    .as_ref()
}
