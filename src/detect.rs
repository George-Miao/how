use std::cmp::Ordering;
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
    pub arbitration: Option<ArbitrationExplanation>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArbitrationExplanation {
    pub selected: CandidateExplanation,
    pub rejected: Vec<CandidateExplanation>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CandidateExplanation {
    pub manager: &'static str,
    pub package: Option<String>,
    pub confidence: Confidence,
    pub phase: &'static str,
    pub mechanism: &'static str,
    pub detail: String,
    pub reason: String,
}

pub fn inspect(command: &OsStr, all: bool, explain: bool) -> Result<Vec<Installation>, Error> {
    resolver::resolve(command, all).map(|paths| {
        paths
            .into_iter()
            .map(|resolution| inspect_path(resolution, explain))
            .collect()
    })
}

fn inspect_path(resolution: Resolution, explain: bool) -> Installation {
    let executable = resolution.executable;
    let resolved = fs::canonicalize(&executable).unwrap_or_else(|_| executable.clone());
    let context = DetectionContext {
        executable: &executable,
        resolved: &resolved,
        probe: command_probe::system(),
    };

    let mut candidates = path_candidates(&context);
    let best_path = CandidateSelectionPolicy::best(&candidates).cloned();
    if best_path
        .as_ref()
        .is_none_or(|candidate| candidate.detection.confidence != Confidence::High)
    {
        collect_ownership_candidates(&context, &mut candidates);
    }
    if candidates.is_empty() {
        candidates.push(Candidate::new(
            Detection {
                manager: "unknown",
                package: None,
                confidence: Confidence::Low,
                mechanism: Mechanism::Inspection,
                detail: "no known path convention or package database matched".into(),
            },
            DetectionPhase::Fallback,
            u16::MAX,
        ));
    }

    let selection =
        CandidateSelectionPolicy::select(candidates).expect("fallback guarantees a candidate");
    let mut evidence = vec![Evidence::from(&selection.selected.detection)];
    if selection.selected.phase == DetectionPhase::Ownership
        && let Some(path) = best_path.as_ref()
    {
        evidence.push(Evidence::from(&path.detection));
    }
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

    let arbitration = explain.then(|| selection.explanation());
    let detection = selection.selected.detection;
    Installation {
        executable,
        resolved,
        manager: detection.manager,
        package: detection.package,
        confidence: detection.confidence,
        evidence,
        arbitration,
    }
}

#[cfg(test)]
fn best_path_detection(context: &DetectionContext<'_>) -> Option<Detection> {
    CandidateSelectionPolicy::select(path_candidates(context))
        .map(|selection| selection.selected.detection)
}

fn path_candidates(context: &DetectionContext<'_>) -> Vec<Candidate> {
    let providers = provider::path_providers();
    debug_assert!(
        providers
            .windows(2)
            .all(|pair| pair[0].priority < pair[1].priority)
    );

    let mut candidates = Vec::new();
    let mut cheap_matches = Vec::with_capacity(providers.len());
    for registration in providers {
        let detection = registration.provider.detect(context);
        cheap_matches.push(detection.is_some());
        if let Some(detection) = detection {
            candidates.push(Candidate::new(
                detection,
                DetectionPhase::CheapPath,
                registration.priority,
            ));
        }
    }

    if CandidateSelectionPolicy::best(&candidates)
        .is_some_and(|candidate| candidate.detection.confidence == Confidence::High)
    {
        return candidates;
    }

    for (registration, cheap_matched) in providers.iter().zip(cheap_matches) {
        if cheap_matched {
            continue;
        }
        let Some(detection) = registration.provider.discover(context) else {
            continue;
        };
        let is_maximum_confidence = detection.confidence == Confidence::High;
        candidates.push(Candidate::new(
            detection,
            DetectionPhase::DynamicPath,
            registration.priority,
        ));
        if is_maximum_confidence {
            break;
        }
    }
    candidates
}

fn collect_ownership_candidates(context: &DetectionContext<'_>, candidates: &mut Vec<Candidate>) {
    let providers = provider::ownership_providers();
    debug_assert!(
        providers
            .windows(2)
            .all(|pair| pair[0].priority < pair[1].priority)
    );

    for registration in providers {
        let Some(detection) = registration.provider.detect(context) else {
            continue;
        };
        let is_maximum_confidence = detection.confidence == Confidence::High;
        candidates.push(Candidate::new(
            detection,
            DetectionPhase::Ownership,
            registration.priority,
        ));
        if is_maximum_confidence {
            break;
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DetectionPhase {
    CheapPath,
    DynamicPath,
    Ownership,
    Fallback,
}

impl DetectionPhase {
    fn as_str(self) -> &'static str {
        match self {
            Self::CheapPath => "cheap path",
            Self::DynamicPath => "dynamic path",
            Self::Ownership => "package database",
            Self::Fallback => "fallback",
        }
    }

    fn rank(self) -> u8 {
        match self {
            Self::CheapPath => 0,
            Self::DynamicPath => 1,
            Self::Ownership => 2,
            Self::Fallback => 3,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Candidate {
    detection: Detection,
    phase: DetectionPhase,
    priority: u16,
}

impl Candidate {
    fn new(detection: Detection, phase: DetectionPhase, priority: u16) -> Self {
        Self {
            detection,
            phase,
            priority,
        }
    }
}

#[derive(Debug)]
struct CandidateSelection {
    selected: Candidate,
    rejected: Vec<RejectedCandidate>,
}

impl CandidateSelection {
    fn explanation(&self) -> ArbitrationExplanation {
        let selected = CandidateExplanation::new(
            &self.selected,
            format!(
                "selected by confidence-first policy; stable priority {}, then phase, mechanism, \
                 manager, package, and evidence break ties",
                self.selected.priority
            ),
        );
        let rejected = self
            .rejected
            .iter()
            .map(|rejected| {
                CandidateExplanation::new(&rejected.candidate, rejected.explanation(&self.selected))
            })
            .collect();
        ArbitrationExplanation { selected, rejected }
    }
}

impl CandidateExplanation {
    fn new(candidate: &Candidate, reason: String) -> Self {
        Self {
            manager: candidate.detection.manager,
            package: candidate.detection.package.clone(),
            confidence: candidate.detection.confidence,
            phase: candidate.phase.as_str(),
            mechanism: mechanism_name(candidate.detection.mechanism),
            detail: candidate.detection.detail.to_string(),
            reason,
        }
    }
}

#[derive(Debug)]
struct RejectedCandidate {
    candidate: Candidate,
    reason: &'static str,
}

impl RejectedCandidate {
    fn explanation(&self, selected: &Candidate) -> String {
        match self.reason {
            "lower confidence" => format!(
                "lower confidence ({}) than selected {} ({})",
                self.candidate.detection.confidence.as_str(),
                selected.detection.manager,
                selected.detection.confidence.as_str()
            ),
            "stable priority" => format!(
                "same confidence; stable priority {} follows selected priority {}",
                self.candidate.priority, selected.priority
            ),
            _ => format!(
                "same confidence and priority; deterministic candidate tie-break ranks after {}",
                selected.detection.manager
            ),
        }
    }
}

struct CandidateSelectionPolicy;

impl CandidateSelectionPolicy {
    fn best(candidates: &[Candidate]) -> Option<&Candidate> {
        candidates
            .iter()
            .min_by(|left, right| Self::compare(left, right))
    }

    fn select(mut candidates: Vec<Candidate>) -> Option<CandidateSelection> {
        if candidates.is_empty() {
            return None;
        }
        candidates.sort_by(Self::compare);
        let selected = candidates.remove(0);
        let rejected = candidates
            .into_iter()
            .map(|candidate| RejectedCandidate {
                reason: rejection_reason(&selected, &candidate),
                candidate,
            })
            .collect();
        Some(CandidateSelection { selected, rejected })
    }

    fn compare(left: &Candidate, right: &Candidate) -> Ordering {
        right
            .detection
            .confidence
            .rank()
            .cmp(&left.detection.confidence.rank())
            .then_with(|| left.priority.cmp(&right.priority))
            .then_with(|| left.phase.rank().cmp(&right.phase.rank()))
            .then_with(|| {
                mechanism_rank(left.detection.mechanism)
                    .cmp(&mechanism_rank(right.detection.mechanism))
            })
            .then_with(|| left.detection.manager.cmp(right.detection.manager))
            .then_with(|| left.detection.package.cmp(&right.detection.package))
            .then_with(|| left.detection.detail.cmp(&right.detection.detail))
    }
}

fn rejection_reason(selected: &Candidate, rejected: &Candidate) -> &'static str {
    if selected.detection.confidence != rejected.detection.confidence {
        "lower confidence"
    } else if selected.priority != rejected.priority {
        "stable priority"
    } else {
        "deterministic tie-break"
    }
}

fn mechanism_name(mechanism: Mechanism) -> &'static str {
    match mechanism {
        Mechanism::PathConvention => "path convention",
        Mechanism::PackageDatabase => "package database",
        Mechanism::Inspection => "inspection",
    }
}

fn mechanism_rank(mechanism: Mechanism) -> u8 {
    match mechanism {
        Mechanism::PathConvention => 0,
        Mechanism::PackageDatabase => 1,
        Mechanism::Inspection => 2,
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
    fn candidate_policy_breaks_confidence_ties_by_stable_priority() {
        let preferred = candidate("preferred", Confidence::High, 10);
        let later = candidate("later", Confidence::High, 20);

        for candidates in [
            vec![preferred.clone(), later.clone()],
            vec![later.clone(), preferred.clone()],
        ] {
            let selection =
                CandidateSelectionPolicy::select(candidates).expect("candidate selection");
            assert_eq!(selection.selected.detection.manager, "preferred");
        }
    }

    #[test]
    fn candidate_policy_prefers_higher_confidence() {
        let selection = CandidateSelectionPolicy::select(vec![
            candidate("priority winner", Confidence::Medium, 10),
            candidate("confidence winner", Confidence::High, 20),
        ])
        .expect("candidate selection");

        assert_eq!(selection.selected.detection.manager, "confidence winner");
    }

    #[test]
    fn candidate_policy_explains_rejected_candidates() {
        let selection = CandidateSelectionPolicy::select(vec![
            candidate("selected", Confidence::High, 10),
            candidate("lower confidence", Confidence::Medium, 5),
            candidate("later priority", Confidence::High, 20),
        ])
        .expect("candidate selection");

        assert_eq!(selection.rejected.len(), 2);
        assert!(
            selection
                .rejected
                .iter()
                .any(|rejected| rejected.reason.contains("lower confidence"))
        );
        assert!(
            selection
                .rejected
                .iter()
                .any(|rejected| rejected.reason.contains("stable priority"))
        );
    }

    fn candidate(manager: &'static str, confidence: Confidence, priority: u16) -> Candidate {
        Candidate::new(
            Detection {
                manager,
                package: None,
                confidence,
                mechanism: Mechanism::PathConvention,
                detail: "test candidate".into(),
            },
            DetectionPhase::CheapPath,
            priority,
        )
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
