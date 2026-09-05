use std::path::Path;

use super::{Confidence, Detection, DetectionContext, Provider};
use crate::util::{self, detect_paths, executable_name};

pub(super) struct Rbenv;
pub(super) static PROVIDER: Rbenv = Rbenv;

impl Provider for Rbenv {
    fn detect(&self, context: &DetectionContext<'_>) -> Option<Detection> {
        let root = util::env_path("RBENV_ROOT")
            .or_else(|| util::home_dir().map(|path| path.join(".rbenv")))?;
        detect_paths(context, |path| detect_under_root(path, &root))
    }

    fn discover(&self, context: &DetectionContext<'_>) -> Option<Detection> {
        let root = util::command_path(context.probe, "rbenv", &["root"])?;
        detect_paths(context, |path| detect_under_root(path, &root))
    }
}

fn detect_under_root(path: &Path, root: &Path) -> Option<Detection> {
    if let Some(version) = util::child_after(path, root, "versions") {
        return Some(Detection::path(
            "rbenv",
            Some(version),
            Confidence::High,
            format!("target lives under rbenv root {}", root.display()),
        ));
    }
    util::executable_is_in(path, &root.join("shims")).then(|| {
        Detection::path(
            "rbenv",
            executable_name(path),
            Confidence::Medium,
            format!("executable is a shim under rbenv root {}", root.display()),
        )
    })
}
