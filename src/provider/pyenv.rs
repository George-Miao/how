use std::path::Path;

use super::{Confidence, Detection, DetectionContext, Provider};
use crate::util::{self, detect_paths, executable_name};

pub(super) struct Pyenv;
pub(super) static PROVIDER: Pyenv = Pyenv;

impl Provider for Pyenv {
    fn detect(&self, context: &DetectionContext<'_>) -> Option<Detection> {
        let root = util::env_path("PYENV_ROOT")
            .or_else(|| util::home_dir().map(|path| path.join(".pyenv")))?;
        detect_paths(context, |path| detect_under_root(path, &root))
    }

    fn discover(&self, context: &DetectionContext<'_>) -> Option<Detection> {
        let root = util::command_path(context.probe, "pyenv", &["root"])?;
        detect_paths(context, |path| detect_under_root(path, &root))
    }
}

fn detect_under_root(path: &Path, root: &Path) -> Option<Detection> {
    if let Some(version) = util::child_after(path, root, "versions") {
        return Some(Detection::toolchain_path(
            "pyenv",
            Some("python".into()),
            Some(version),
            Confidence::High,
            format!("target lives under pyenv root {}", root.display()),
        ));
    }
    util::executable_is_in(path, &root.join("shims")).then(|| {
        Detection::toolchain_path(
            "pyenv",
            executable_name(path),
            None,
            Confidence::Medium,
            format!("executable is a shim under pyenv root {}", root.display()),
        )
    })
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
        assert_eq!(detection.provenance.toolchain.as_deref(), Some("python"));
        assert_eq!(detection.provenance.version.as_deref(), Some("3.13"));
        assert_eq!(detection.provenance.package, None);
    }
}
