use std::path::{Path, PathBuf};

use super::{Confidence, Detection, DetectionContext, Provider};
use crate::util::{self, detect_paths, executable_name};

pub(super) struct Pipx;
pub(super) static PROVIDER: Pipx = Pipx;

struct Locations {
    homes: Vec<PathBuf>,
    explicit_bins: Vec<PathBuf>,
}

impl Provider for Pipx {
    fn detect(&self, context: &DetectionContext<'_>) -> Option<Detection> {
        let locations = configured_locations();
        detect_paths(context, |path| detect_path(path, &locations))
    }

    fn discover(&self, context: &DetectionContext<'_>) -> Option<Detection> {
        if let Some(home) = pipx_value(context.probe, "PIPX_HOME") {
            let locations = Locations {
                homes: vec![home],
                explicit_bins: Vec::new(),
            };
            if let Some(detection) = detect_paths(context, |path| detect_path(path, &locations)) {
                return Some(detection);
            }
        }
        let home = pipx_global_value(context.probe, "PIPX_GLOBAL_HOME")?;
        let locations = Locations {
            homes: vec![home],
            explicit_bins: Vec::new(),
        };
        detect_paths(context, |path| detect_path(path, &locations))
    }
}

fn detect_path(path: &Path, locations: &Locations) -> Option<Detection> {
    for home in &locations.homes {
        if let Some(package) = util::child_after(path, home, "venvs") {
            return Some(Detection::path(
                "pipx",
                Some(package),
                Confidence::High,
                format!("target lives in pipx home {}", home.display()),
            ));
        }
    }
    let bin = locations
        .explicit_bins
        .iter()
        .find(|bin| util::executable_is_in(path, bin))?;
    Some(Detection::path(
        "pipx",
        executable_name(path),
        Confidence::Medium,
        format!(
            "executable is in configured pipx bin directory {}",
            bin.display()
        ),
    ))
}

fn configured_locations() -> Locations {
    let mut homes = Vec::new();
    push_unique(
        &mut homes,
        util::env_path("PIPX_HOME").or_else(default_home),
    );
    push_unique(&mut homes, util::env_path("PIPX_GLOBAL_HOME"));
    if let Some(home) = util::home_dir() {
        push_unique(&mut homes, Some(home.join(".local/pipx")));
    }

    let mut explicit_bins = Vec::new();
    push_unique(&mut explicit_bins, util::env_path("PIPX_BIN_DIR"));
    push_unique(&mut explicit_bins, util::env_path("PIPX_GLOBAL_BIN_DIR"));
    Locations {
        homes,
        explicit_bins,
    }
}

fn pipx_value(probe: &dyn crate::command_probe::CommandProbe, name: &str) -> Option<PathBuf> {
    util::command_path(probe, "pipx", &["environment", "--value", name])
}

fn pipx_global_value(
    probe: &dyn crate::command_probe::CommandProbe,
    name: &str,
) -> Option<PathBuf> {
    util::command_path(probe, "pipx", &["environment", "--global", "--value", name])
}

fn default_home() -> Option<PathBuf> {
    cfg_select! {
        target_os = "macos" => {
            util::home_dir().map(|home| home.join("Library/Application Support/pipx"))
        }
        windows => util::env_path("LOCALAPPDATA").map(|home| home.join("pipx")),
        _ => util::env_path("XDG_DATA_HOME")
            .map(|path| path.join("pipx"))
            .or_else(|| util::home_dir().map(|home| home.join(".local/share/pipx"))),
    }
}

fn push_unique(paths: &mut Vec<PathBuf>, path: Option<PathBuf>) {
    if let Some(path) = path
        && !paths.contains(&path)
    {
        paths.push(path);
    }
}
