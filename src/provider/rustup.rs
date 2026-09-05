use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use super::{Confidence, Detection, DetectionContext, Provider};
use crate::command_probe::{CommandProbe, CommandSpec};
use crate::util;

pub(super) struct Rustup;
pub(super) static PROVIDER: Rustup = Rustup;

impl Provider for Rustup {
    fn detect(&self, context: &DetectionContext<'_>) -> Option<Detection> {
        let rustup_home = rustup_home()?;
        util::detect_paths(context, |path| detect_toolchain_path(path, &rustup_home))
    }

    fn discover(&self, context: &DetectionContext<'_>) -> Option<Detection> {
        let rustup_home = rustup_home()?;
        let cargo_home = cargo_home()?;
        detect_proxy(context.executable, &cargo_home, &rustup_home, context.probe).or_else(|| {
            (context.executable != context.resolved)
                .then(|| detect_proxy(context.resolved, &cargo_home, &rustup_home, context.probe))
                .flatten()
        })
    }
}

fn rustup_home() -> Option<PathBuf> {
    util::env_path("RUSTUP_HOME").or_else(|| util::home_dir().map(|home| home.join(".rustup")))
}

fn cargo_home() -> Option<PathBuf> {
    util::env_path("CARGO_HOME").or_else(|| util::home_dir().map(|home| home.join(".cargo")))
}

fn detect_toolchain_path(path: &Path, rustup_home: &Path) -> Option<Detection> {
    let relative = util::relative_to(path, &rustup_home.join("toolchains"))?;
    let mut components = relative.components();
    let toolchain = components
        .next()?
        .as_os_str()
        .to_string_lossy()
        .into_owned();
    let bin = components.next()?.as_os_str();
    let executable = components.next()?;
    if toolchain.is_empty()
        || !bin.to_string_lossy().eq_ignore_ascii_case("bin")
        || executable.as_os_str().is_empty()
        || components.next().is_some()
    {
        return None;
    }

    Some(Detection::toolchain_path(
        "Rustup",
        Some(toolchain),
        None,
        Confidence::High,
        format!(
            "target lives in a toolchain under Rustup home {}",
            rustup_home.display()
        ),
    ))
}

fn detect_proxy(
    path: &Path,
    cargo_home: &Path,
    rustup_home: &Path,
    probe: &dyn CommandProbe,
) -> Option<Detection> {
    if !util::executable_is_in(path, &cargo_home.join("bin")) {
        return None;
    }
    let proxy = normalized_executable_name(path)?;
    if proxy.eq_ignore_ascii_case("rustup") {
        return verify_rustup(path, probe);
    }
    if !is_standard_proxy(proxy) {
        return None;
    }

    let rustup = cargo_home.join("bin").join(rustup_executable_name());
    let output = probe.output(CommandSpec::new(rustup).args(["which", proxy]))?;
    let selected = PathBuf::from(String::from_utf8_lossy(&output).trim());
    if !normalized_executable_name(&selected)?.eq_ignore_ascii_case(proxy) {
        return None;
    }
    let direct = detect_toolchain_path(&selected, rustup_home)?;
    Some(Detection::toolchain_inspection(
        "Rustup",
        direct.provenance.toolchain,
        Confidence::High,
        format!(
            "rustup which {proxy} selected a toolchain binary under {}",
            rustup_home.display()
        ),
    ))
}

fn verify_rustup(path: &Path, probe: &dyn CommandProbe) -> Option<Detection> {
    let output = probe.output(CommandSpec::new(path).arg("--version"))?;
    let version = String::from_utf8_lossy(&output);
    version
        .split_whitespace()
        .next()
        .is_some_and(|program| program.eq_ignore_ascii_case("rustup"))
        .then(|| {
            Detection::toolchain_inspection(
                "Rustup",
                None,
                Confidence::High,
                "rustup --version verified the Rustup proxy",
            )
        })
}

fn normalized_executable_name(path: &Path) -> Option<&str> {
    let name = path.file_name()?.to_str()?;
    Some(
        name.strip_suffix(".exe")
            .or_else(|| name.strip_suffix(".EXE"))
            .unwrap_or(name),
    )
}

fn rustup_executable_name() -> &'static OsStr {
    if cfg!(windows) {
        OsStr::new("rustup.exe")
    } else {
        OsStr::new("rustup")
    }
}

fn is_standard_proxy(name: &str) -> bool {
    [
        "cargo",
        "cargo-clippy",
        "cargo-fmt",
        "cargo-miri",
        "clippy-driver",
        "rls",
        "rust-analyzer",
        "rust-gdb",
        "rust-gdbgui",
        "rust-lldb",
        "rustc",
        "rustdoc",
        "rustfmt",
    ]
    .iter()
    .any(|proxy| name.eq_ignore_ascii_case(proxy))
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;
    use crate::command_probe::{CommandProbe, CommandSpec};

    struct StubProbe {
        output: Option<Arc<[u8]>>,
        calls: AtomicUsize,
    }

    impl StubProbe {
        fn returning(output: impl Into<Vec<u8>>) -> Self {
            Self {
                output: Some(Arc::from(output.into())),
                calls: AtomicUsize::new(0),
            }
        }

        fn calls(&self) -> usize {
            self.calls.load(Ordering::Relaxed)
        }
    }

    impl CommandProbe for StubProbe {
        fn output(&self, _command: CommandSpec) -> Option<Arc<[u8]>> {
            self.calls.fetch_add(1, Ordering::Relaxed);
            self.output.clone()
        }
    }

    fn fixture_path(components: &[&str]) -> PathBuf {
        components.iter().collect()
    }

    fn path_output(path: &Path) -> Vec<u8> {
        format!("{}\n", path.display()).into_bytes()
    }

    #[test]
    fn detects_direct_toolchain_binary_with_structured_provenance() {
        let detection = detect_toolchain_path(
            Path::new("/srv/rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc"),
            Path::new("/srv/rustup"),
        )
        .unwrap();

        assert_eq!(detection.manager, "Rustup");
        assert_eq!(
            detection.provenance.toolchain.as_deref(),
            Some("nightly-x86_64-unknown-linux-gnu")
        );
        assert_eq!(detection.provenance.package, None);
        assert_eq!(detection.confidence, Confidence::High);
    }

    #[test]
    fn rejects_paths_outside_the_toolchain_bin_directory() {
        assert!(
            detect_toolchain_path(
                Path::new("/srv/rustup/toolchains/nightly/lib/rustlib"),
                Path::new("/srv/rustup"),
            )
            .is_none()
        );
    }

    #[test]
    fn verifies_a_standard_proxy_and_reports_selected_toolchain() {
        let cargo_home = fixture_path(&["srv", "cargo"]);
        let rustup_home = fixture_path(&["srv", "rustup"]);
        let selected = rustup_home
            .join("toolchains")
            .join("stable-x86_64-unknown-linux-gnu")
            .join("bin")
            .join("rustc");
        let probe = StubProbe::returning(path_output(&selected));
        let detection = detect_proxy(
            &cargo_home.join("bin").join("rustc"),
            &cargo_home,
            &rustup_home,
            &probe,
        )
        .unwrap();

        assert_eq!(detection.manager, "Rustup");
        assert_eq!(
            detection.provenance.toolchain.as_deref(),
            Some("stable-x86_64-unknown-linux-gnu")
        );
        assert_eq!(detection.confidence, Confidence::High);
        assert_eq!(probe.calls(), 1);
    }

    #[test]
    fn verifies_rustup_itself() {
        let cargo_home = fixture_path(&["srv", "cargo"]);
        let rustup_home = fixture_path(&["srv", "rustup"]);
        let probe = StubProbe::returning(b"rustup 1.28.2 (e4f3ad6f8 2025-04-28)\n".to_vec());
        let detection = detect_proxy(
            &cargo_home.join("bin").join("rustup"),
            &cargo_home,
            &rustup_home,
            &probe,
        )
        .unwrap();

        assert_eq!(detection.manager, "Rustup");
        assert_eq!(detection.provenance.toolchain, None);
        assert_eq!(detection.confidence, Confidence::High);
        assert_eq!(probe.calls(), 1);
    }

    #[test]
    fn rejects_an_unverified_standard_proxy() {
        let cargo_home = fixture_path(&["srv", "cargo"]);
        let rustup_home = fixture_path(&["srv", "rustup"]);
        let probe = StubProbe::returning(path_output(&fixture_path(&["usr", "bin", "rustc"])));

        assert!(
            detect_proxy(
                &cargo_home.join("bin").join("rustc"),
                &cargo_home,
                &rustup_home,
                &probe,
            )
            .is_none()
        );
    }

    #[test]
    fn arbitrary_cargo_installed_binary_is_not_probed_or_claimed() {
        let cargo_home = fixture_path(&["srv", "cargo"]);
        let rustup_home = fixture_path(&["srv", "rustup"]);
        let selected = rustup_home
            .join("toolchains")
            .join("stable-x86_64-unknown-linux-gnu")
            .join("bin")
            .join("ripgrep");
        let probe = StubProbe::returning(path_output(&selected));

        assert!(
            detect_proxy(
                &cargo_home.join("bin").join("rg"),
                &cargo_home,
                &rustup_home,
                &probe,
            )
            .is_none()
        );
        assert_eq!(probe.calls(), 0);
    }
}
