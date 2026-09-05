use std::ffi::OsStr;
use std::fs;
use std::path::PathBuf;

use crate::command_probe;
use crate::error::Error;
use crate::provider::{self, Confidence, Detection, DetectionContext, Evidence, Mechanism};
use crate::resolver::{self, Resolution};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Installation {
    pub executable: PathBuf,
    pub resolved: PathBuf,
    pub manager: &'static str,
    pub package: Option<String>,
    pub confidence: Confidence,
    pub evidence: Vec<Evidence>,
}

pub fn inspect(command: &OsStr, all: bool) -> Result<Vec<Installation>, Error> {
    resolver::resolve(command, all).map(|paths| paths.into_iter().map(inspect_path).collect())
}

fn inspect_path(resolution: Resolution) -> Installation {
    let executable = resolution.executable;
    let resolved = fs::canonicalize(&executable).unwrap_or_else(|_| executable.clone());
    let context = DetectionContext {
        executable: &executable,
        resolved: &resolved,
        probe: command_probe::system(),
    };

    let path_detection = best_path_detection(&context);

    let ownership = if path_detection
        .as_ref()
        .is_none_or(|detection| detection.confidence != Confidence::High)
    {
        provider::ownership_providers()
            .iter()
            .find_map(|provider| provider.detect(&context))
    } else {
        None
    };

    let (detection, mut evidence) = select_detection(ownership, path_detection);
    for alias in resolution.aliases.into_iter().rev() {
        evidence.insert(
            0,
            Evidence {
                kind: "shell alias",
                detail: format!(
                    "{} expands to {:?}",
                    alias.name.to_string_lossy(),
                    alias.value
                ),
            },
        );
    }
    if resolved != executable {
        evidence.push(Evidence {
            kind: "symlink",
            detail: format!("target is {}", resolved.display()),
        });
    }

    Installation {
        executable,
        resolved,
        manager: detection.manager,
        package: detection.package,
        confidence: detection.confidence,
        evidence,
    }
}

fn best_path_detection(context: &DetectionContext<'_>) -> Option<Detection> {
    let providers = provider::path_providers();
    let mut cheap = Vec::with_capacity(providers.len());
    for provider in providers {
        let detection = provider.detect(context);
        if detection
            .as_ref()
            .is_some_and(|detection| detection.confidence == Confidence::High)
        {
            return detection;
        }
        cheap.push(detection);
    }

    let mut best = None;
    for (provider, detection) in providers.iter().zip(cheap) {
        let Some(detection) = detection.or_else(|| provider.discover(context)) else {
            continue;
        };
        if detection.confidence == Confidence::High {
            return Some(detection);
        }
        if best.as_ref().is_none_or(|current: &Detection| {
            detection.confidence.rank() > current.confidence.rank()
        }) {
            best = Some(detection);
        }
    }
    best
}

fn select_detection(
    ownership: Option<Detection>,
    path_detection: Option<Detection>,
) -> (Detection, Vec<Evidence>) {
    match (ownership, path_detection) {
        (Some(ownership), path_detection) => {
            let mut evidence = vec![Evidence::from(&ownership)];
            if let Some(path_detection) = path_detection {
                evidence.push(Evidence::from(&path_detection));
            }
            (ownership, evidence)
        }
        (None, Some(path_detection)) => {
            let evidence = vec![Evidence::from(&path_detection)];
            (path_detection, evidence)
        }
        (None, None) => {
            let detection = Detection {
                manager: "unknown",
                package: None,
                confidence: Confidence::Low,
                mechanism: Mechanism::Inspection,
                detail: "no known path convention or package database matched".into(),
            };
            let evidence = vec![Evidence::from(&detection)];
            (detection, evidence)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::command_probe::{CommandProbe, CommandSpec};

    struct RejectingProbe;

    impl CommandProbe for RejectingProbe {
        fn output(&self, _command: CommandSpec) -> Option<std::sync::Arc<[u8]>> {
            panic!("static detection invoked a subprocess probe");
        }
    }

    #[test]
    fn static_high_confidence_path_skips_subprocess_probes() {
        let executable = Path::new("/snap/bin/firefox");
        let context = DetectionContext {
            executable,
            resolved: executable,
            probe: &RejectingProbe,
        };

        let detection = best_path_detection(&context).expect("Snap detection");

        assert_eq!(detection.manager, "Snap");
        assert_eq!(detection.confidence, Confidence::High);
    }

    #[test]
    fn detects_msys2_clang64_executable_by_path() {
        let executable = Path::new("D:/msys64/clang64/bin/rg.exe");
        let context = DetectionContext {
            executable,
            resolved: executable,
            probe: crate::command_probe::system(),
        };

        let detection = best_path_detection(&context).expect("MSYS2 detection");
        assert_eq!(detection.manager, "MSYS2 pacman");
    }
}
