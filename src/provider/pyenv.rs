use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use super::{Confidence, Detection, DetectionContext, Provider};
use crate::util::{self, detect_paths, executable_name};

pub(super) struct Pyenv;
pub(super) static PROVIDER: Pyenv = Pyenv;
static ROOT: OnceLock<Option<PathBuf>> = OnceLock::new();

impl Provider for Pyenv {
    fn detect(&self, context: &DetectionContext<'_>) -> Option<Detection> {
        detect_paths(context, detect_path)
    }
}

fn detect_path(path: &Path) -> Option<Detection> {
    detect_under_root(path, root()?)
}

fn detect_under_root(path: &Path, root: &Path) -> Option<Detection> {
    if let Some(version) = util::child_after(path, root, "versions") {
        return Some(Detection::path(
            "pyenv",
            Some(version),
            Confidence::High,
            format!("target lives under pyenv root {}", root.display()),
        ));
    }
    util::executable_is_in(path, &root.join("shims")).then(|| {
        Detection::path(
            "pyenv",
            executable_name(path),
            Confidence::Medium,
            format!("executable is a shim under pyenv root {}", root.display()),
        )
    })
}

fn root() -> Option<&'static PathBuf> {
    ROOT.get_or_init(|| {
        util::env_path("PYENV_ROOT")
            .or_else(|| util::command_path("pyenv", &["root"]))
            .or_else(|| util::home_dir().map(|path| path.join(".pyenv")))
    })
    .as_ref()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supports_configured_root() {
        let detection = detect_under_root(
            Path::new("/srv/python/versions/3.13/bin/python"),
            Path::new("/srv/python"),
        )
        .unwrap();
        assert_eq!(detection.package.as_deref(), Some("3.13"));
    }
}
