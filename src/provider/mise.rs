use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use super::{Confidence, Detection, DetectionContext, Provider};
use crate::util::{self, detect_paths, executable_name};

pub(super) struct Mise;
pub(super) static PROVIDER: Mise = Mise;
static ROOT: OnceLock<Option<PathBuf>> = OnceLock::new();

impl Provider for Mise {
    fn detect(&self, context: &DetectionContext<'_>) -> Option<Detection> {
        detect_paths(context, detect_path)
    }
}

fn detect_path(path: &Path) -> Option<Detection> {
    detect_under_root(path, root()?)
}

fn detect_under_root(path: &Path, root: &Path) -> Option<Detection> {
    if let Some((toolchain, version)) = util::first_two_components(path, &root.join("installs")) {
        return Some(Detection::toolchain_path(
            "mise",
            Some(toolchain),
            version,
            Confidence::High,
            format!("target lives in mise data directory {}", root.display()),
        ));
    }
    util::executable_is_in(path, &root.join("shims")).then(|| {
        Detection::toolchain_path(
            "mise",
            executable_name(path),
            None,
            Confidence::Medium,
            format!("executable is a shim in {}", root.join("shims").display()),
        )
    })
}

fn root() -> Option<&'static PathBuf> {
    ROOT.get_or_init(|| {
        util::env_path("MISE_DATA_DIR")
            .or_else(|| util::env_path("XDG_DATA_HOME").map(|path| path.join("mise")))
            .or_else(|| util::home_dir().map(|path| path.join(".local/share/mise")))
    })
    .as_ref()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_toolchain_and_version() {
        let detection = detect_under_root(
            Path::new("/opt/mise/installs/python/3.13.0/bin/python"),
            Path::new("/opt/mise"),
        )
        .unwrap();

        assert_eq!(detection.provenance.toolchain.as_deref(), Some("python"));
        assert_eq!(detection.provenance.version.as_deref(), Some("3.13.0"));
        assert_eq!(detection.provenance.package, None);
    }
}
