use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use super::{Confidence, Detection, DetectionContext, Provider};
use crate::util::{self, detect_paths, executable_name};

pub(super) struct Composer;
pub(super) static PROVIDER: Composer = Composer;
static HOME: OnceLock<Option<PathBuf>> = OnceLock::new();

impl Provider for Composer {
    fn detect(&self, context: &DetectionContext<'_>) -> Option<Detection> {
        detect_paths(context, detect_path)
    }
}

fn detect_path(path: &Path) -> Option<Detection> {
    let bin = home()?.join("vendor/bin");
    util::executable_is_in(path, &bin).then(|| {
        Detection::path(
            "Composer",
            executable_name(path),
            Confidence::Medium,
            format!(
                "executable is in Composer global bin directory {}",
                bin.display()
            ),
        )
    })
}

fn home() -> Option<&'static PathBuf> {
    HOME.get_or_init(|| {
        util::env_path("COMPOSER_HOME")
            .or_else(|| util::home_dir().map(|home| home.join(".composer")))
    })
    .as_ref()
}
