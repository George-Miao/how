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

/// Provider precedence used after confidence; lower values win ties.
///
/// Registries are kept in ascending priority so a maximum-confidence match can
/// stop later dynamic probes without changing the selected result.
#[derive(Clone, Copy)]
pub(crate) struct ProviderRegistration {
    pub provider: &'static dyn Provider,
    pub priority: u16,
}

impl ProviderRegistration {
    const fn new(priority: u16, provider: &'static dyn Provider) -> Self {
        Self { provider, priority }
    }
}

static PATH_PROVIDERS: &[ProviderRegistration] = &[
    ProviderRegistration::new(100, &nix::PROVIDER),
    ProviderRegistration::new(200, &homebrew::PROVIDER),
    ProviderRegistration::new(300, &snap::PROVIDER),
    ProviderRegistration::new(400, &flatpak::PROVIDER),
    ProviderRegistration::new(500, &pixi::PROVIDER),
    ProviderRegistration::new(600, &conda::PROVIDER),
    ProviderRegistration::new(700, &mise::PROVIDER),
    ProviderRegistration::new(800, &asdf::PROVIDER),
    ProviderRegistration::new(900, &pyenv::PROVIDER),
    ProviderRegistration::new(1000, &rbenv::PROVIDER),
    ProviderRegistration::new(1100, &volta::PROVIDER),
    ProviderRegistration::new(1200, &uv::PROVIDER),
    ProviderRegistration::new(1300, &pipx::PROVIDER),
    ProviderRegistration::new(1400, &pnpm::PROVIDER),
    ProviderRegistration::new(1500, &npm::PROVIDER),
    ProviderRegistration::new(1600, &yarn::PROVIDER),
    ProviderRegistration::new(1700, &bun::PROVIDER),
    ProviderRegistration::new(1800, &deno::PROVIDER),
    ProviderRegistration::new(1900, &composer::PROVIDER),
    ProviderRegistration::new(2000, &cargo::PROVIDER),
    ProviderRegistration::new(2100, &go::PROVIDER),
    ProviderRegistration::new(2200, &msys2::PROVIDER),
    ProviderRegistration::new(2300, &winget::PROVIDER),
    ProviderRegistration::new(2400, &scoop::PROVIDER),
    ProviderRegistration::new(2500, &chocolatey::PROVIDER),
    ProviderRegistration::new(2600, &windows_apps::PROVIDER),
    ProviderRegistration::new(2700, &macports::PROVIDER),
    ProviderRegistration::new(2800, &system::PROVIDER),
];

pub(crate) fn path_providers() -> &'static [ProviderRegistration] {
    PATH_PROVIDERS
}

static OWNERSHIP_PROVIDERS: &[ProviderRegistration] = &[
    ProviderRegistration::new(3000, &dpkg::PROVIDER),
    ProviderRegistration::new(3100, &rpm::PROVIDER),
    ProviderRegistration::new(3200, &pacman::PROVIDER),
    ProviderRegistration::new(3300, &apk::PROVIDER),
    ProviderRegistration::new(3400, &freebsd_pkg::PROVIDER),
];

pub(crate) fn ownership_providers() -> &'static [ProviderRegistration] {
    OWNERSHIP_PROVIDERS
}
