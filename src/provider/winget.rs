use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use super::{Confidence, Detection, DetectionContext, Provider};
use crate::util::{self, detect_paths, executable_name};

pub(super) struct WinGet;
pub(super) static PROVIDER: WinGet = WinGet;
static LOCATIONS: OnceLock<Locations> = OnceLock::new();

struct Locations {
    package_roots: Vec<PathBuf>,
    link_root: Option<PathBuf>,
}

impl Provider for WinGet {
    fn detect(&self, context: &DetectionContext<'_>) -> Option<Detection> {
        detect_paths(context, detect_path)
    }
}

fn detect_path(path: &Path) -> Option<Detection> {
    let locations = locations();
    for root in &locations.package_roots {
        if let Some(package) = package_under(path, root) {
            return Some(Detection::path(
                "WinGet",
                Some(package),
                Confidence::High,
                format!("target lives in WinGet package root {}", root.display()),
            ));
        }
    }
    let links = locations.link_root.as_ref()?;
    util::executable_is_in(path, links).then(|| {
        Detection::path(
            "WinGet",
            executable_name(path),
            Confidence::Medium,
            format!(
                "executable is in WinGet links directory {}",
                links.display()
            ),
        )
    })
}

fn locations() -> &'static Locations {
    LOCATIONS.get_or_init(|| {
        let local = util::env_path("LOCALAPPDATA");
        let mut package_roots = Vec::new();
        if let Some(root) = local.as_ref() {
            package_roots.push(root.join("Microsoft/WinGet/Packages"));
        }
        if let Some(root) = util::env_path("PROGRAMFILES") {
            package_roots.push(root.join("WinGet/Packages"));
        }
        Locations {
            package_roots,
            link_root: local.map(|root| root.join("Microsoft/WinGet/Links")),
        }
    })
}

fn package_under(path: &Path, root: &Path) -> Option<String> {
    let relative = util::relative_to(path, root)?;
    let directory = relative.components().next()?;
    let directory = directory.as_os_str().to_string_lossy();
    Some(
        directory
            .split_once('_')
            .map(|(package, _)| package)
            .unwrap_or(&directory)
            .to_owned(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_portable_package_id() {
        assert_eq!(
            package_under(
                Path::new(
                    "C:/Users/me/AppData/Local/Microsoft/WinGet/Packages/BurntSushi.ripgrep.\
                     MSVC_Microsoft.Winget.Source_8wekyb3d8bbwe/rg.exe"
                ),
                Path::new("C:/Users/me/AppData/Local/Microsoft/WinGet/Packages"),
            )
            .as_deref(),
            Some("BurntSushi.ripgrep.MSVC")
        );
    }
}
