use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use super::{Confidence, Detection, DetectionContext, Provider};
use crate::util::{self, detect_paths, executable_name};

pub(super) struct Uv;
pub(super) static PROVIDER: Uv = Uv;
static LOCATIONS: OnceLock<Locations> = OnceLock::new();

struct Locations {
    tools: Option<PathBuf>,
    explicit_bin: Option<PathBuf>,
}

impl Provider for Uv {
    fn detect(&self, context: &DetectionContext<'_>) -> Option<Detection> {
        detect_paths(context, detect_path)
    }
}

fn detect_path(path: &Path) -> Option<Detection> {
    let locations = locations();
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

fn locations() -> &'static Locations {
    LOCATIONS.get_or_init(|| Locations {
        tools: util::env_path("UV_TOOL_DIR")
            .or_else(|| util::command_path("uv", &["tool", "dir"]))
            .or_else(default_tools_dir),
        // The default executable directory is shared by several installers, so
        // only an explicit override is strong enough to use as evidence.
        explicit_bin: util::env_path("UV_TOOL_BIN_DIR"),
    })
}

fn default_tools_dir() -> Option<PathBuf> {
    util::env_path("XDG_DATA_HOME")
        .map(|path| path.join("uv/tools"))
        .or_else(|| util::home_dir().map(|path| path.join(".local/share/uv/tools")))
}
