use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use super::{Confidence, Detection, DetectionContext, Provider};
use crate::util::{self, detect_paths, executable_name};

pub(super) struct Cargo;
pub(super) static PROVIDER: Cargo = Cargo;
static INSTALL_ROOT: OnceLock<Option<PathBuf>> = OnceLock::new();

impl Provider for Cargo {
    fn detect(&self, context: &DetectionContext<'_>) -> Option<Detection> {
        detect_paths(context, detect_path)
    }
}

fn detect_path(path: &Path) -> Option<Detection> {
    let root = install_root()?;
    util::executable_is_in(path, &root.join("bin")).then(|| {
        Detection::path(
            "Cargo",
            executable_name(path),
            Confidence::Medium,
            format!("executable is in Cargo install root {}", root.display()),
        )
    })
}

fn install_root() -> Option<&'static PathBuf> {
    INSTALL_ROOT
        .get_or_init(|| {
            util::env_path("CARGO_INSTALL_ROOT").or_else(|| {
                let cargo_home = util::env_path("CARGO_HOME")
                    .or_else(|| util::home_dir().map(|home| home.join(".cargo")))?;
                cargo_config_root(&cargo_home).or(Some(cargo_home))
            })
        })
        .as_ref()
}

fn cargo_config_root(cargo_home: &Path) -> Option<PathBuf> {
    [cargo_home.join("config.toml"), cargo_home.join("config")]
        .into_iter()
        .find_map(|path| {
            let root = fs::read_to_string(&path)
                .ok()
                .and_then(|text| parse_install_root(&text))?;
            Some(if root.is_absolute() {
                root
            } else {
                path.parent()?.join(root)
            })
        })
}

fn parse_install_root(config: &str) -> Option<PathBuf> {
    let mut in_install = false;
    for raw_line in config.lines() {
        let line = raw_line.split('#').next()?.trim();
        if line.starts_with('[') {
            in_install = line == "[install]";
            continue;
        }
        let value = if in_install {
            let Some(value) = line.strip_prefix("root") else {
                continue;
            };
            let Some(value) = value.trim_start().strip_prefix('=') else {
                continue;
            };
            value
        } else if let Some(value) = line.strip_prefix("install.root") {
            value.trim_start().strip_prefix('=')?
        } else {
            continue;
        };
        let value = value.trim().trim_matches(['\"', '\'']);
        if !value.is_empty() {
            return Some(PathBuf::from(value));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_install_root_from_cargo_config() {
        assert_eq!(
            parse_install_root("[install]\nroot = '/opt/cargo-tools'\n").as_deref(),
            Some(Path::new("/opt/cargo-tools"))
        );
    }
}
