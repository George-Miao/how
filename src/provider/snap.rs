use std::path::Path;

use super::{Confidence, Detection, DetectionContext, Provider};
use crate::util::{components, detect_paths, executable_name, position};

pub(super) struct Snap;
pub(super) static PROVIDER: Snap = Snap;

impl Provider for Snap {
    fn detect(&self, context: &DetectionContext<'_>) -> Option<Detection> {
        detect_paths(context, detect_path)
    }
}

fn detect_path(path: &Path) -> Option<Detection> {
    let components = components(path);
    if position(&components, &["snap", "bin"]).is_some() {
        return Some(Detection::path(
            "Snap",
            executable_name(path),
            Confidence::High,
            "executable is exposed through a Snap bin directory",
        ));
    }
    let index = components
        .iter()
        .position(|component| component == "snap")?;
    let package = components.get(index + 1)?;
    (package != "bin").then(|| {
        Detection::path(
            "Snap",
            Some(package.clone()),
            Confidence::High,
            "target lives in a mounted Snap package",
        )
    })
}
