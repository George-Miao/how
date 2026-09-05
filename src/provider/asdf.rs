use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use super::{Confidence, Detection, DetectionContext, Provider};
use crate::util::{self, detect_paths, executable_name};

pub(super) struct Asdf;
pub(super) static PROVIDER: Asdf = Asdf;
static ROOT: OnceLock<Option<PathBuf>> = OnceLock::new();

impl Provider for Asdf {
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
            "asdf",
            Some(toolchain),
            version,
            Confidence::High,
            format!("target lives in asdf data directory {}", root.display()),
        ));
    }
    util::executable_is_in(path, &root.join("shims")).then(|| {
        Detection::toolchain_path(
            "asdf",
            executable_name(path),
            None,
            Confidence::Medium,
            format!("executable is a shim in {}", root.join("shims").display()),
        )
    })
}

fn root() -> Option<&'static PathBuf> {
    ROOT.get_or_init(|| {
        util::env_path("ASDF_DATA_DIR").or_else(|| util::home_dir().map(|path| path.join(".asdf")))
    })
    .as_ref()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_toolchain_and_version() {
        let detection = detect_under_root(
            Path::new("/opt/asdf/installs/nodejs/22.0.0/bin/node"),
            Path::new("/opt/asdf"),
        )
        .unwrap();

        assert_eq!(detection.provenance.toolchain.as_deref(), Some("nodejs"));
        assert_eq!(detection.provenance.version.as_deref(), Some("22.0.0"));
        assert_eq!(detection.provenance.package, None);
    }
}
