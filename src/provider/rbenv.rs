use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use super::{Confidence, Detection, DetectionContext, Provider};
use crate::util::{self, detect_paths, executable_name};

pub(super) struct Rbenv;
pub(super) static PROVIDER: Rbenv = Rbenv;
static ROOT: OnceLock<Option<PathBuf>> = OnceLock::new();

impl Provider for Rbenv {
    fn detect(&self, context: &DetectionContext<'_>) -> Option<Detection> {
        detect_paths(context, detect_path)
    }
}

fn detect_path(path: &Path) -> Option<Detection> {
    let root = root()?;
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

fn root() -> Option<&'static PathBuf> {
    ROOT.get_or_init(|| {
        util::env_path("RBENV_ROOT")
            .or_else(|| util::command_path("rbenv", &["root"]))
            .or_else(|| util::home_dir().map(|path| path.join(".rbenv")))
    })
    .as_ref()
}
