mod apk;
mod asdf;
mod bun;
mod cargo;
mod chocolatey;
mod composer;
mod conda;
mod deno;
mod dpkg;
mod flatpak;
mod freebsd_pkg;
mod go;
mod homebrew;
mod macports;
mod mise;
mod msys2;
mod nix;
mod npm;
mod pacman;
mod pipx;
mod pixi;
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
use std::path::Path;

use crate::command_probe::CommandProbe;

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

    pub(super) fn inspection(
        manager: &'static str,
        package: Option<String>,
        confidence: Confidence,
        detail: impl Into<Cow<'static, str>>,
    ) -> Self {
        Self {
            manager,
            package,
            confidence,
            mechanism: Mechanism::Inspection,
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
    pub probe: &'a dyn CommandProbe,
}

/// A source of package-manager provenance evidence.
///
/// `detect` checks path conventions and configured or default roots. `discover`
/// may run provider commands and is deferred until no static high-confidence
/// detection matched.
pub trait Provider: Send + Sync {
    fn detect(&self, context: &DetectionContext<'_>) -> Option<Detection>;

    fn discover(&self, _context: &DetectionContext<'_>) -> Option<Detection> {
        None
    }
}

static PATH_PROVIDERS: &[&dyn Provider] = &[
    &nix::PROVIDER,
    &homebrew::PROVIDER,
    &snap::PROVIDER,
    &flatpak::PROVIDER,
    &pixi::PROVIDER,
    &conda::PROVIDER,
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
    &msys2::PROVIDER,
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
