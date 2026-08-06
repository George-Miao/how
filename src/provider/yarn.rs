use std::path::Path;

use super::{Confidence, Detection, DetectionContext, Provider};
use crate::util::{components, detect_paths, executable_name};

pub(super) struct Yarn;
pub(super) static PROVIDER: Yarn = Yarn;

impl Provider for Yarn {
    fn detect(&self, context: &DetectionContext<'_>) -> Option<Detection> {
        detect_paths(context, detect_path)
    }
}

fn detect_path(path: &Path) -> Option<Detection> {
    components(path)
        .iter()
        .any(|component| component == ".yarn")
        .then(|| {
            Detection::path(
                "Yarn",
                executable_name(path),
                Confidence::Medium,
                "target lives in a Yarn-managed directory",
            )
        })
}
