use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use super::{Confidence, Detection, DetectionContext, Provider};
use crate::util::{self, detect_paths, executable_name};

pub(super) struct Scoop;
pub(super) static PROVIDER: Scoop = Scoop;
static ROOTS: OnceLock<Vec<PathBuf>> = OnceLock::new();

impl Provider for Scoop {
    fn detect(&self, context: &DetectionContext<'_>) -> Option<Detection> {
        detect_paths(context, detect_path)
    }
}

fn detect_path(path: &Path) -> Option<Detection> {
    for root in roots() {
        if let Some(detection) = detect_under_root(path, root) {
            return Some(detection);
        }
    }
    None
}

fn detect_under_root(path: &Path, root: &Path) -> Option<Detection> {
    if let Some(package) = util::child_after(path, root, "apps") {
        return Some(Detection::path(
            "Scoop",
            Some(package),
            Confidence::High,
            format!("target lives under Scoop root {}", root.display()),
        ));
    }
    util::executable_is_in(path, &root.join("shims")).then(|| {
        Detection::path(
            "Scoop",
            executable_name(path),
            Confidence::Medium,
            format!("executable is a shim under Scoop root {}", root.display()),
        )
    })
}

fn roots() -> &'static [PathBuf] {
    ROOTS.get_or_init(|| {
        let mut roots = Vec::new();
        if let Some(root) =
            util::env_path("SCOOP").or_else(|| util::home_dir().map(|home| home.join("scoop")))
        {
            roots.push(root);
        }
        if let Some(root) = util::env_path("SCOOP_GLOBAL")
            .or_else(|| util::env_path("PROGRAMDATA").map(|path| path.join("scoop")))
            && !roots.contains(&root)
        {
            roots.push(root);
        }
        roots
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_app_from_configured_root() {
        let detection = detect_under_root(
            Path::new("C:/portable/scoop/apps/ripgrep/current/rg.exe"),
            Path::new("C:/portable/scoop"),
        )
        .unwrap();
        assert_eq!(detection.provenance.package.as_deref(), Some("ripgrep"));
    }
}
