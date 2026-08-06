use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use super::{Confidence, Detection, DetectionContext, Provider};
use crate::util::{self, components, detect_paths, executable_name, position};

pub(super) struct Pnpm;
pub(super) static PROVIDER: Pnpm = Pnpm;
static GLOBAL_BIN: OnceLock<Option<PathBuf>> = OnceLock::new();

impl Provider for Pnpm {
    fn detect(&self, context: &DetectionContext<'_>) -> Option<Detection> {
        detect_paths(context, detect_path)
    }
}

fn detect_path(path: &Path) -> Option<Detection> {
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

    detect_global_bin(path, global_bin()?)
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

fn global_bin() -> Option<&'static PathBuf> {
    GLOBAL_BIN
        .get_or_init(|| {
            util::env_path("PNPM_HOME").or_else(|| util::command_path("pnpm", &["bin", "--global"]))
        })
        .as_ref()
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
