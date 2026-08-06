use std::path::Path;

use super::{Confidence, Detection, DetectionContext, Provider};
use crate::util::{components, detect_paths, executable_name, position};

pub(super) struct Flatpak;
pub(super) static PROVIDER: Flatpak = Flatpak;

impl Provider for Flatpak {
    fn detect(&self, context: &DetectionContext<'_>) -> Option<Detection> {
        detect_paths(context, detect_path)
    }
}

fn detect_path(path: &Path) -> Option<Detection> {
    position(&components(path), &["flatpak", "exports", "bin"])?;
    Some(Detection::path(
        "Flatpak",
        executable_name(path),
        Confidence::High,
        "executable is in a Flatpak exports directory",
    ))
}
