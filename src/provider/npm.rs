use std::path::Path;

use super::{Confidence, Detection, DetectionContext, Provider};
use crate::util::{self, components, detect_paths, executable_name};

pub(super) struct Npm;
pub(super) static PROVIDER: Npm = Npm;

impl Provider for Npm {
    fn detect(&self, context: &DetectionContext<'_>) -> Option<Detection> {
        let prefix = util::env_path("NPM_CONFIG_PREFIX");
        detect_paths(context, |path| detect_path(path, prefix.as_deref()))
    }

    fn discover(&self, context: &DetectionContext<'_>) -> Option<Detection> {
        let prefix = util::command_path(context.probe, "npm", &["prefix", "--global"])?;
        detect_paths(context, |path| detect_prefix(path, &prefix))
    }
}

fn detect_path(path: &Path, prefix: Option<&Path>) -> Option<Detection> {
    let components = components(path);
    if !components.iter().any(|component| component == ".pnpm")
        && let Some(index) = components
            .iter()
            .rposition(|component| component == "node_modules")
    {
        return Some(Detection::path(
            "npm-compatible package manager",
            node_package(&components, index),
            Confidence::Medium,
            "target lives in node_modules",
        ));
    }

    detect_prefix(path, prefix?)
}

fn detect_prefix(path: &Path, prefix: &Path) -> Option<Detection> {
    let bin = prefix.join("bin");
    util::executable_is_in(path, &bin).then(|| {
        Detection::path(
            "npm",
            executable_name(path),
            Confidence::Medium,
            format!("executable is under npm global prefix {}", bin.display()),
        )
    })
}

fn node_package(components: &[String], node_modules_index: usize) -> Option<String> {
    let first = components.get(node_modules_index + 1)?;
    if first == ".bin" {
        return None;
    }
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
    fn extracts_scoped_package() {
        let detection = detect_path(
            Path::new("/repo/node_modules/@scope/tool/bin/tool.js"),
            None,
        )
        .unwrap();
        assert_eq!(detection.provenance.package.as_deref(), Some("@scope/tool"));
    }
}
