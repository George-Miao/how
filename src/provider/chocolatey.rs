use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use super::{Confidence, Detection, DetectionContext, Provider};
use crate::util::{self, detect_paths, executable_name};

pub(super) struct Chocolatey;
pub(super) static PROVIDER: Chocolatey = Chocolatey;
static ROOT: OnceLock<Option<PathBuf>> = OnceLock::new();

impl Provider for Chocolatey {
    fn detect(&self, context: &DetectionContext<'_>) -> Option<Detection> {
        detect_paths(context, detect_path)
    }
}

fn detect_path(path: &Path) -> Option<Detection> {
    detect_under_root(path, root()?)
}

fn detect_under_root(path: &Path, root: &Path) -> Option<Detection> {
    if let Some(package) = util::child_after(path, root, "lib") {
        return Some(Detection::path(
            "Chocolatey",
            Some(package),
            Confidence::High,
            format!("target lives under Chocolatey root {}", root.display()),
        ));
    }
    util::executable_is_in(path, &root.join("bin")).then(|| {
        Detection::path(
            "Chocolatey",
            executable_name(path),
            Confidence::Medium,
            format!(
                "executable is a shim under Chocolatey root {}",
                root.display()
            ),
        )
    })
}

fn root() -> Option<&'static PathBuf> {
    ROOT.get_or_init(|| {
        util::env_path("ChocolateyInstall")
            .or_else(|| util::env_path("PROGRAMDATA").map(|path| path.join("Chocolatey")))
    })
    .as_ref()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_package_from_configured_root() {
        let detection = detect_under_root(
            Path::new("C:/tools/choco/lib/ripgrep/tools/rg.exe"),
            Path::new("C:/tools/choco"),
        )
        .unwrap();
        assert_eq!(detection.package.as_deref(), Some("ripgrep"));
    }
}
