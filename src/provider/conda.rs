use std::path::Path;

use super::{Confidence, Detection, DetectionContext, Provider};
use crate::util::detect_paths;

pub(super) struct Conda;
pub(super) static PROVIDER: Conda = Conda;

impl Provider for Conda {
    fn detect(&self, context: &DetectionContext<'_>) -> Option<Detection> {
        detect_paths(context, detect_path)
    }
}

fn detect_path(path: &Path) -> Option<Detection> {
    let prefix = conda_prefix(path)?;
    Some(Detection::environment_inspection(
        "Conda-compatible",
        environment_name(prefix),
        Confidence::High,
        format!(
            "target belongs to Conda-compatible prefix {}",
            prefix.display()
        ),
    ))
}

fn conda_prefix(path: &Path) -> Option<&Path> {
    path.ancestors()
        .find(|ancestor| ancestor.join("conda-meta").is_dir())
}

fn environment_name(prefix: &Path) -> Option<String> {
    let parent = prefix.parent()?;
    if !parent
        .file_name()
        .is_some_and(|name| name.to_string_lossy().eq_ignore_ascii_case("envs"))
    {
        return None;
    }
    prefix
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    #[test]
    fn detects_named_environment_from_metadata() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("how-conda-{}-{nonce}", std::process::id()));
        let prefix = root.join("envs/science");
        fs::create_dir_all(prefix.join("conda-meta")).unwrap();

        let detection = detect_path(&prefix.join("bin/python")).unwrap();

        assert_eq!(detection.manager, "Conda-compatible");
        assert_eq!(detection.provenance.environment.as_deref(), Some("science"));
        assert_eq!(detection.provenance.package, None);
        assert_eq!(detection.confidence, Confidence::High);
        fs::remove_dir_all(root).unwrap();
    }
}
