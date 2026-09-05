use std::path::Path;

use super::{Confidence, Detection, DetectionContext, Provider};
use crate::util::{self, components, detect_paths};

pub(super) struct Homebrew;
pub(super) static PROVIDER: Homebrew = Homebrew;

impl Provider for Homebrew {
    fn detect(&self, context: &DetectionContext<'_>) -> Option<Detection> {
        let cellar = util::env_path("HOMEBREW_CELLAR");
        detect_paths(context, |path| detect_path(path, cellar.as_deref()))
    }

    fn discover(&self, context: &DetectionContext<'_>) -> Option<Detection> {
        let cellar = util::command_path(context.probe, "brew", &["--cellar"])?;
        detect_paths(context, |path| detect_configured(path, &cellar))
    }
}

fn detect_path(path: &Path, configured: Option<&Path>) -> Option<Detection> {
    if let Some(cellar) = configured
        && let Some(detection) = detect_configured(path, cellar)
    {
        return Some(detection);
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

fn detect_configured(path: &Path, cellar: &Path) -> Option<Detection> {
    let formula = child(path, cellar)?;
    Some(Detection::path(
        "Homebrew",
        Some(formula),
        Confidence::High,
        format!(
            "target lives in configured Homebrew Cellar {}",
            cellar.display()
        ),
    ))
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
        let detection = detect_path(
            Path::new("/opt/homebrew/Cellar/ripgrep/14.1.1/bin/rg"),
            None,
        )
        .unwrap();
        assert_eq!(detection.package.as_deref(), Some("ripgrep"));
    }
}
