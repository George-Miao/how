use std::path::{Path, PathBuf};

use super::{Confidence, Detection, DetectionContext, Provider};
use crate::util::{self, detect_paths, executable_name};

pub(super) struct Uv;
pub(super) static PROVIDER: Uv = Uv;

struct Locations {
    tools: Option<PathBuf>,
    explicit_bin: Option<PathBuf>,
}

impl Provider for Uv {
    fn detect(&self, context: &DetectionContext<'_>) -> Option<Detection> {
        let locations = configured_locations();
        detect_paths(context, |path| detect_path(path, &locations))
    }

    fn discover(&self, context: &DetectionContext<'_>) -> Option<Detection> {
        let locations = Locations {
            tools: util::command_path(context.probe, "uv", &["tool", "dir"]),
            explicit_bin: None,
        };
        detect_paths(context, |path| detect_path(path, &locations))
    }
}

fn detect_path(path: &Path, locations: &Locations) -> Option<Detection> {
    if let Some(root) = &locations.tools
        && let Some(package) = path
            .strip_prefix(root)
            .ok()
            .and_then(|relative| relative.components().next())
    {
        return Some(Detection::path(
            "uv",
            Some(package.as_os_str().to_string_lossy().into_owned()),
            Confidence::High,
            format!("target lives in uv tools directory {}", root.display()),
        ));
    }
    let bin = locations.explicit_bin.as_ref()?;
    util::executable_is_in(path, bin).then(|| {
        Detection::path(
            "uv",
            executable_name(path),
            Confidence::Medium,
            format!(
                "executable is in configured UV_TOOL_BIN_DIR {}",
                bin.display()
            ),
        )
    })
}

fn configured_locations() -> Locations {
    Locations {
        tools: util::env_path("UV_TOOL_DIR").or_else(default_tools_dir),
        // The default executable directory is shared by several installers, so
        // only an explicit override is strong enough to use as evidence.
        explicit_bin: util::env_path("UV_TOOL_BIN_DIR"),
    }
}

fn default_tools_dir() -> Option<PathBuf> {
    util::env_path("XDG_DATA_HOME")
        .map(|path| path.join("uv/tools"))
        .or_else(|| util::home_dir().map(|path| path.join(".local/share/uv/tools")))
}
