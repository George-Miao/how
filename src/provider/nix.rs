use std::path::Path;

use super::{Confidence, Detection, DetectionContext, Provider};
use crate::util::{self, components, detect_paths, position};

pub(super) struct Nix;
pub(super) static PROVIDER: Nix = Nix;

impl Provider for Nix {
    fn detect(&self, context: &DetectionContext<'_>) -> Option<Detection> {
        detect_paths(context, detect_path)
    }
}

fn detect_path(path: &Path) -> Option<Detection> {
    if let Some(store) = util::env_path("NIX_STORE_DIR")
        && let Some(derivation) = store_derivation(path, &store)
    {
        return Some(Detection::derivation_path(
            "Nix",
            derivation,
            Confidence::High,
            format!("target lives in configured Nix store {}", store.display()),
        ));
    }

    let components = components(path);
    let index = position(&components, &["nix", "store"])?;
    let derivation = store_name(components.get(index + 2)?);
    Some(Detection::derivation_path(
        "Nix",
        derivation,
        Confidence::High,
        "target lives in /nix/store",
    ))
}

fn store_derivation(path: &Path, store: &Path) -> Option<String> {
    let entry = path.strip_prefix(store).ok()?.components().next()?;
    Some(store_name(&entry.as_os_str().to_string_lossy()))
}

fn store_name(entry: &str) -> String {
    entry
        .split_once('-')
        .map(|(_, name)| name)
        .unwrap_or(entry)
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_store_entry_name() {
        let detection = detect_path(Path::new("/nix/store/abc123-ripgrep-14.1.1/bin/rg")).unwrap();
        assert_eq!(
            detection.provenance.derivation.as_deref(),
            Some("ripgrep-14.1.1")
        );
        assert_eq!(detection.provenance.package, None);
    }
}
