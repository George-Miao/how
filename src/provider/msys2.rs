use std::path::Path;

use super::{Confidence, Detection, DetectionContext, Provider};
use crate::util::{self, detect_paths};

pub(super) struct Msys2;
pub(super) static PROVIDER: Msys2 = Msys2;

impl Provider for Msys2 {
    fn detect(&self, context: &DetectionContext<'_>) -> Option<Detection> {
        let location = location(context.resolved).or_else(|| location(context.executable))?;
        let pacman = location.root.join("usr/bin/pacman.exe");
        if pacman.is_file()
            && let Some(ownership) = util::query_ownership(
                context,
                &pacman,
                &["-Qqo"],
                "MSYS2 pacman",
                util::first_line,
            )
        {
            return Some(ownership);
        }

        detect_paths(context, detect_path)
    }
}

struct Location<'a> {
    root: &'a Path,
    environment: &'static str,
}

fn location(path: &Path) -> Option<Location<'_>> {
    let bin = path.parent()?;
    if !name_is(bin, "bin") {
        return None;
    }
    let prefix = bin.parent()?;
    let environment = match prefix.file_name()?.to_string_lossy().as_ref() {
        name if name.eq_ignore_ascii_case("usr") => "MSYS",
        name if name.eq_ignore_ascii_case("ucrt64") => "UCRT64",
        name if name.eq_ignore_ascii_case("clang64") => "CLANG64",
        name if name.eq_ignore_ascii_case("clangarm64") => "CLANGARM64",
        name if name.eq_ignore_ascii_case("mingw32") => "MINGW32",
        name if name.eq_ignore_ascii_case("mingw64") => "MINGW64",
        name if name.eq_ignore_ascii_case("clang32") => "CLANG32",
        _ => return None,
    };
    Some(Location {
        root: prefix.parent()?,
        environment,
    })
}

fn name_is(path: &Path, expected: &str) -> bool {
    path.file_name()
        .is_some_and(|name| name.to_string_lossy().eq_ignore_ascii_case(expected))
}

fn detect_path(path: &Path) -> Option<Detection> {
    let location = location(path)?;
    Some(Detection::path(
        "MSYS2 pacman",
        None,
        Confidence::High,
        format!(
            "executable is in the MSYS2 {} prefix under {}",
            location.environment,
            location.root.display()
        ),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_custom_root_and_environment() {
        let location = location(Path::new("D:/msys64/clang64/bin/rg.exe")).unwrap();
        assert_eq!(location.root, Path::new("D:/msys64"));
        assert_eq!(location.environment, "CLANG64");
    }
}
