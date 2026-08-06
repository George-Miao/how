use std::env;
use std::ffi::OsStr;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

use crate::provider::{Detection, DetectionContext};

pub fn detect_paths(
    context: &DetectionContext<'_>,
    detector: impl Fn(&Path) -> Option<Detection>,
) -> Option<Detection> {
    let resolved = detector(context.resolved);
    let invoked = (context.executable != context.resolved)
        .then(|| detector(context.executable))
        .flatten();
    match (resolved, invoked) {
        (Some(resolved), Some(invoked))
            if invoked.confidence.rank() > resolved.confidence.rank() =>
        {
            Some(invoked)
        }
        (Some(resolved), _) => Some(resolved),
        (None, invoked) => invoked,
    }
}

pub fn components(path: &Path) -> Vec<String> {
    path.components()
        .map(|component| match component {
            Component::RootDir => String::new(),
            _ => component.as_os_str().to_string_lossy().into_owned(),
        })
        .collect()
}

pub fn position(haystack: &[String], needle: &[&str]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window.iter().zip(needle).all(|(left, right)| left == right))
}

pub fn executable_name(path: &Path) -> Option<String> {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
}

pub fn env_path(name: &str) -> Option<PathBuf> {
    env::var_os(name)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

pub fn home_dir() -> Option<PathBuf> {
    env_path("HOME").or_else(|| env_path("USERPROFILE"))
}

pub fn executable_is_in(path: &Path, directory: &Path) -> bool {
    path.parent()
        .is_some_and(|parent| paths_equal(parent, directory))
}

pub fn paths_equal(left: &Path, right: &Path) -> bool {
    let lexically_equal = cfg_select! {
        windows => left
            .as_os_str()
            .to_string_lossy()
            .eq_ignore_ascii_case(&right.as_os_str().to_string_lossy()),
        _ => left == right,
    };
    if lexically_equal {
        return true;
    }
    match (left.canonicalize(), right.canonicalize()) {
        (Ok(left), Ok(right)) => left == right,
        _ => false,
    }
}

pub fn relative_to(path: &Path, root: &Path) -> Option<PathBuf> {
    cfg_select! {
        windows => {
            let path_components: Vec<_> = path.components().collect();
            let root_components: Vec<_> = root.components().collect();
            let starts_with_root =
                path_components
                    .iter()
                    .zip(&root_components)
                    .all(|(left, right)| {
                        left.as_os_str()
                            .to_string_lossy()
                            .eq_ignore_ascii_case(&right.as_os_str().to_string_lossy())
                    });
            (starts_with_root && path_components.len() >= root_components.len()).then(|| {
                path_components[root_components.len()..]
                    .iter()
                    .map(|component| component.as_os_str())
                    .collect()
            })
        }
        _ => path.strip_prefix(root).ok().map(PathBuf::from),
    }
}

pub fn command_path(program: &str, arguments: &[&str]) -> Option<PathBuf> {
    command_output(program, arguments).map(PathBuf::from)
}

pub fn command_output(program: &str, arguments: &[&str]) -> Option<String> {
    let output = Command::new(program).args(arguments).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    (!value.is_empty()).then_some(value)
}

pub fn path_list(value: &OsStr) -> Vec<PathBuf> {
    env::split_paths(value).collect()
}

pub fn child_after(path: &Path, root: &Path, child: &str) -> Option<String> {
    relative_to(path, &root.join(child))?
        .components()
        .next()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
}

pub fn query_ownership(
    context: &DetectionContext<'_>,
    program: impl AsRef<OsStr>,
    arguments: &[&str],
    manager: &'static str,
    parser: fn(&str) -> Option<String>,
) -> Option<Detection> {
    let output = Command::new(program)
        .args(arguments)
        .arg(context.resolved)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let package = parser(stdout.trim())?;
    Some(Detection::ownership(
        manager,
        package.clone(),
        format!("{manager} reports ownership by {package}"),
    ))
}

pub fn first_line(output: &str) -> Option<String> {
    let line = output.lines().next()?.trim();
    (!line.is_empty()).then(|| line.to_owned())
}
