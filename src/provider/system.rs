use std::path::Path;

use super::{Confidence, Detection, DetectionContext, Provider};
use crate::util::{components, detect_paths};

pub(super) struct System;
pub(super) static PROVIDER: System = System;

impl Provider for System {
    fn detect(&self, context: &DetectionContext<'_>) -> Option<Detection> {
        detect_paths(context, detect_path)
    }
}

fn detect_path(path: &Path) -> Option<Detection> {
    let path = components(path).join("/");
    let is_system = matches!(path.as_str(), "/usr/bin" | "/usr/sbin" | "/bin" | "/sbin")
        || path.starts_with("/usr/bin/")
        || path.starts_with("/usr/sbin/")
        || path.starts_with("/bin/")
        || path.starts_with("/sbin/");
    is_system.then(|| {
        Detection::path(
            "system package manager",
            None,
            Confidence::Low,
            "executable is in a system bin directory",
        )
    })
}
