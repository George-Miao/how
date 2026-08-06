use std::ffi::OsStr;
use std::fs;
use std::path::PathBuf;

use crate::error::Error;
use crate::provider::{self, Confidence, Detection, DetectionContext, Evidence, Mechanism};
use crate::resolver;

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

fn inspect_path(executable: PathBuf) -> Installation {
    let resolved = fs::canonicalize(&executable).unwrap_or_else(|_| executable.clone());
    let context = DetectionContext {
        executable: &executable,
        resolved: &resolved,
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
    let mut best = None;
    for provider in provider::path_providers() {
        let Some(detection) = provider.detect(context) else {
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
