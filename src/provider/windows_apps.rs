use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use super::{Confidence, Detection, DetectionContext, Provider};
use crate::util::{self, detect_paths, executable_name};

pub(super) struct WindowsApps;
pub(super) static PROVIDER: WindowsApps = WindowsApps;
static ROOT: OnceLock<Option<PathBuf>> = OnceLock::new();

impl Provider for WindowsApps {
    fn detect(&self, context: &DetectionContext<'_>) -> Option<Detection> {
        detect_paths(context, detect_path)
    }
}

fn detect_path(path: &Path) -> Option<Detection> {
    let root = root()?;
    util::executable_is_in(path, root).then(|| {
        Detection::path(
            "Microsoft Store or App Installer",
            executable_name(path),
            Confidence::Low,
            format!(
                "executable is a Windows app execution alias in {}",
                root.display()
            ),
        )
    })
}

fn root() -> Option<&'static PathBuf> {
    ROOT.get_or_init(|| {
        util::env_path("LOCALAPPDATA").map(|path| path.join("Microsoft/WindowsApps"))
    })
    .as_ref()
}
