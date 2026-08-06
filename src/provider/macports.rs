use std::path::Path;

use super::{Confidence, Detection, DetectionContext, Provider};
use crate::util::{components, detect_paths};

pub(super) struct MacPorts;
pub(super) static PROVIDER: MacPorts = MacPorts;

impl Provider for MacPorts {
    fn detect(&self, context: &DetectionContext<'_>) -> Option<Detection> {
        detect_paths(context, detect_path)
    }
}

fn detect_path(path: &Path) -> Option<Detection> {
    components(path)
        .starts_with(&[String::new(), "opt".into(), "local".into()])
        .then(|| {
            Detection::path(
                "MacPorts",
                None,
                Confidence::Medium,
                "executable is under MacPorts' /opt/local prefix",
            )
        })
}
