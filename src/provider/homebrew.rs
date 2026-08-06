use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use super::{Confidence, Detection, DetectionContext, Provider};
use crate::util::{self, components, detect_paths};

pub(super) struct Homebrew;
pub(super) static PROVIDER: Homebrew = Homebrew;
static CELLAR: OnceLock<Option<PathBuf>> = OnceLock::new();

impl Provider for Homebrew {
    fn detect(&self, context: &DetectionContext<'_>) -> Option<Detection> {
        detect_paths(context, detect_path)
    }
}

fn detect_path(path: &Path) -> Option<Detection> {
    if let Some(cellar) = configured_cellar()
        && let Some(formula) = child(path, cellar)
    {
        return Some(Detection::path(
            "Homebrew",
            Some(formula),
            Confidence::High,
            format!(
                "target lives in configured Homebrew Cellar {}",
                cellar.display()
            ),
        ));
    }

    let components = components(path);
    let index = components
        .iter()
        .position(|component| component == "Cellar")?;
    Some(Detection::path(
        "Homebrew",
        components.get(index + 1).cloned(),
        Confidence::High,
        "target lives in a Homebrew Cellar",
    ))
}

fn configured_cellar() -> Option<&'static PathBuf> {
    CELLAR
        .get_or_init(|| {
            util::env_path("HOMEBREW_CELLAR").or_else(|| util::command_path("brew", &["--cellar"]))
        })
        .as_ref()
}

fn child(path: &Path, root: &Path) -> Option<String> {
    path.strip_prefix(root)
        .ok()?
        .components()
        .next()
        .map(|value| value.as_os_str().to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_formula() {
        let detection =
            detect_path(Path::new("/opt/homebrew/Cellar/ripgrep/14.1.1/bin/rg")).unwrap();
        assert_eq!(detection.package.as_deref(), Some("ripgrep"));
    }
}
