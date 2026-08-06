mod apk;
mod asdf;
mod bun;
mod cargo;
mod chocolatey;
mod composer;
mod deno;
mod dpkg;
mod flatpak;
mod freebsd_pkg;
mod go;
mod homebrew;
mod macports;
mod mise;
mod nix;
mod npm;
mod pacman;
mod pipx;
mod pnpm;
mod pyenv;
mod rbenv;
mod rpm;
mod scoop;
mod snap;
mod system;
mod uv;
mod volta;
mod windows_apps;
mod winget;
mod yarn;

use std::borrow::Cow;
use std::fmt;
use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Confidence {
    High,
    Medium,
    Low,
}

impl Confidence {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::High => "high",
            Self::Medium => "medium",
            Self::Low => "low",
        }
    }

    pub(crate) fn rank(self) -> u8 {
        match self {
            Self::High => 3,
            Self::Medium => 2,
            Self::Low => 1,
        }
    }
}

impl fmt::Display for Confidence {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} confidence", self.as_str())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mechanism {
    PathConvention,
    PackageDatabase,
    Inspection,
}

impl Mechanism {
    fn as_str(self) -> &'static str {
        match self {
            Self::PathConvention => "path convention",
            Self::PackageDatabase => "package database",
            Self::Inspection => "inspection",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Detection {
    pub manager: &'static str,
    pub package: Option<String>,
    pub confidence: Confidence,
    pub mechanism: Mechanism,
    pub detail: Cow<'static, str>,
}

impl Detection {
    pub(super) fn path(
        manager: &'static str,
        package: Option<String>,
        confidence: Confidence,
        detail: impl Into<Cow<'static, str>>,
    ) -> Self {
        Self {
            manager,
            package,
            confidence,
            mechanism: Mechanism::PathConvention,
            detail: detail.into(),
        }
    }

    pub(super) fn ownership(
        manager: &'static str,
        package: String,
        detail: impl Into<Cow<'static, str>>,
    ) -> Self {
        Self {
            manager,
            package: Some(package),
            confidence: Confidence::High,
            mechanism: Mechanism::PackageDatabase,
            detail: detail.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Evidence {
    pub kind: &'static str,
    pub detail: String,
}

impl From<&Detection> for Evidence {
    fn from(detection: &Detection) -> Self {
        Self {
            kind: detection.mechanism.as_str(),
            detail: detection.detail.to_string(),
        }
    }
}

pub struct DetectionContext<'a> {
    pub executable: &'a Path,
    pub resolved: &'a Path,
}

/// A source of package-manager provenance evidence.
///
/// Providers hide whether they inspect a path convention, invoke an ownership
/// database, or use another mechanism added later.
pub trait Provider: Send + Sync {
    fn detect(&self, context: &DetectionContext<'_>) -> Option<Detection>;
}

static PATH_PROVIDERS: &[&dyn Provider] = &[
    &nix::PROVIDER,
    &homebrew::PROVIDER,
    &snap::PROVIDER,
    &flatpak::PROVIDER,
    &mise::PROVIDER,
    &asdf::PROVIDER,
    &pyenv::PROVIDER,
    &rbenv::PROVIDER,
    &volta::PROVIDER,
    &uv::PROVIDER,
    &pipx::PROVIDER,
    &pnpm::PROVIDER,
    &npm::PROVIDER,
    &yarn::PROVIDER,
    &bun::PROVIDER,
    &deno::PROVIDER,
    &composer::PROVIDER,
    &cargo::PROVIDER,
    &go::PROVIDER,
    &winget::PROVIDER,
    &scoop::PROVIDER,
    &chocolatey::PROVIDER,
    &windows_apps::PROVIDER,
    &macports::PROVIDER,
    &system::PROVIDER,
];

pub fn path_providers() -> &'static [&'static dyn Provider] {
    PATH_PROVIDERS
}

static OWNERSHIP_PROVIDERS: &[&dyn Provider] = &[
    &dpkg::PROVIDER,
    &rpm::PROVIDER,
    &pacman::PROVIDER,
    &apk::PROVIDER,
    &freebsd_pkg::PROVIDER,
];

pub fn ownership_providers() -> &'static [&'static dyn Provider] {
    OWNERSHIP_PROVIDERS
}
