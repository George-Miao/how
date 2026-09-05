use std::path::Path;

use super::{Confidence, Detection, DetectionContext, Provider};
use crate::util::{self, components, detect_paths, executable_name, position};

pub(super) struct Pnpm;
pub(super) static PROVIDER: Pnpm = Pnpm;

impl Provider for Pnpm {
    fn detect(&self, context: &DetectionContext<'_>) -> Option<Detection> {
        let global_bin = util::env_path("PNPM_HOME");
        detect_paths(context, |path| detect_path(path, global_bin.as_deref()))
    }

    fn discover(&self, context: &DetectionContext<'_>) -> Option<Detection> {
        let global_bin = util::command_path(context.probe, "pnpm", &["bin", "--global"])?;
        detect_paths(context, |path| detect_global_bin(path, &global_bin))
    }
}

fn detect_path(path: &Path, global_bin: Option<&Path>) -> Option<Detection> {
    let components = components(path);
    if components.iter().any(|component| component == ".pnpm") {
        let package = components
            .iter()
            .rposition(|component| component == "node_modules")
            .and_then(|index| node_package(&components, index));
        return Some(Detection::path(
            "pnpm",
            package,
            Confidence::High,
            "target lives in pnpm's virtual store layout",
        ));
    }

    if position(&components, &[".local", "share", "pnpm"]).is_some()
        || position(&components, &["Library", "pnpm"]).is_some()
    {
        return Some(Detection::path(
            "pnpm",
            executable_name(path),
            Confidence::Medium,
            "executable is in a conventional pnpm global bin directory",
        ));
    }

    detect_global_bin(path, global_bin?)
}

fn detect_global_bin(path: &Path, global_bin: &Path) -> Option<Detection> {
    util::executable_is_in(path, global_bin).then(|| {
        Detection::path(
            "pnpm",
            executable_name(path),
            Confidence::Medium,
            format!(
                "executable is in pnpm global bin directory {}",
                global_bin.display()
            ),
        )
    })
}

fn node_package(components: &[String], node_modules_index: usize) -> Option<String> {
    let first = components.get(node_modules_index + 1)?;
    if first.starts_with('@') {
        components
            .get(node_modules_index + 2)
            .map(|second| format!("{first}/{second}"))
    } else {
        Some(first.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_configured_global_bin() {
        let detection = detect_global_bin(
            Path::new("/custom/pnpm/bin/tool"),
            Path::new("/custom/pnpm/bin"),
        )
        .unwrap();
        assert_eq!(detection.manager, "pnpm");
        assert_eq!(detection.package.as_deref(), Some("tool"));
    }
}
