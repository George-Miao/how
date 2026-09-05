use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use super::{Confidence, Detection, DetectionContext, Provider};
use crate::util::{self, components, detect_paths, executable_name, position};

pub(super) struct Pixi;
pub(super) static PROVIDER: Pixi = Pixi;
static HOME: OnceLock<Option<PathBuf>> = OnceLock::new();

impl Provider for Pixi {
    fn detect(&self, context: &DetectionContext<'_>) -> Option<Detection> {
        detect_paths(context, detect_path)
    }
}

fn detect_path(path: &Path) -> Option<Detection> {
    let path_components = components(path);
    if let Some(index) = position(&path_components, &[".pixi", "envs"])
        && let Some(environment) = path_components.get(index + 2)
    {
        return Some(Detection::environment_path(
            "Pixi",
            Some(environment.clone()),
            Confidence::High,
            "target lives in a Pixi workspace environment",
        ));
    }

    detect_under_home(path, home()?)
}

fn detect_under_home(path: &Path, home: &Path) -> Option<Detection> {
    if let Some(environment) = util::child_after(path, home, "envs") {
        return Some(Detection::environment_path(
            "Pixi",
            Some(environment),
            Confidence::High,
            format!("target lives in Pixi home {}", home.display()),
        ));
    }
    util::executable_is_in(path, &home.join("bin")).then(|| {
        Detection::path(
            "Pixi",
            executable_name(path),
            Confidence::Medium,
            format!("executable is exposed by Pixi home {}", home.display()),
        )
    })
}

fn home() -> Option<&'static PathBuf> {
    HOME.get_or_init(|| {
        util::env_path("PIXI_HOME").or_else(|| util::home_dir().map(|home| home.join(".pixi")))
    })
    .as_ref()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_workspace_environment() {
        let detection = detect_path(Path::new("/work/project/.pixi/envs/test/bin/pytest")).unwrap();
        assert_eq!(detection.manager, "Pixi");
        assert_eq!(detection.provenance.environment.as_deref(), Some("test"));
        assert_eq!(detection.provenance.package, None);
        assert_eq!(detection.confidence, Confidence::High);
    }

    #[test]
    fn detects_global_environment_under_configured_home() {
        let detection = detect_under_home(
            Path::new("/opt/pixi/envs/science/bin/python"),
            Path::new("/opt/pixi"),
        )
        .unwrap();
        assert_eq!(detection.provenance.environment.as_deref(), Some("science"));
        assert_eq!(detection.provenance.package, None);
        assert_eq!(detection.confidence, Confidence::High);
    }
}
