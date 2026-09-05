use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

static NEXT_TEMP_DIR: AtomicU64 = AtomicU64::new(0);

struct TestFs {
    root: PathBuf,
}

impl TestFs {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock is before Unix epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "how-cli-{}-{nonce}-{}",
            std::process::id(),
            NEXT_TEMP_DIR.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).expect("create isolated test directory");
        Self { root }
    }

    fn executable(&self, relative: impl AsRef<Path>) -> PathBuf {
        let path = self.root.join(relative);
        fs::create_dir_all(path.parent().expect("fixture executable has a parent"))
            .expect("create fixture directory");
        fs::copy(how_binary(), &path).expect("copy executable fixture");
        path
    }
}

impl Drop for TestFs {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).expect("remove isolated test directory");
    }
}

fn how_binary() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_how"))
}

fn fixture_name(name: &str) -> OsString {
    if cfg!(windows) {
        format!("{name}.exe").into()
    } else {
        name.into()
    }
}

fn run_how(path: impl IntoIterator<Item = impl AsRef<OsStr>>, args: &[&OsStr]) -> Output {
    let path = std::env::join_paths(path).expect("fixture PATH is representable");
    let mut command = Command::new(how_binary());
    command.env_clear().env("PATH", path).args(args);

    #[cfg(windows)]
    for name in ["SystemRoot", "WINDIR"] {
        if let Some(value) = std::env::var_os(name) {
            command.env(name, value);
        }
    }

    command.output().expect("execute how binary")
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "status: {:?}\nstdout: {}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn json_output(output: &Output) -> Value {
    assert_success(output);
    serde_json::from_slice(&output.stdout).expect("stdout is one JSON document")
}

fn assert_path_eq(actual: &Path, expected: &Path) {
    #[cfg(windows)]
    assert!(
        actual
            .as_os_str()
            .to_string_lossy()
            .eq_ignore_ascii_case(&expected.as_os_str().to_string_lossy()),
        "paths differ: {} != {}",
        actual.display(),
        expected.display()
    );

    #[cfg(not(windows))]
    assert_eq!(actual, expected);
}

#[test]
fn resolves_a_command_from_a_controlled_path() {
    let fs = TestFs::new();
    let executable = fs.executable(Path::new("bin").join(fixture_name("fixture")));
    let output = run_how(
        [executable.parent().unwrap()],
        &[OsStr::new("--json"), OsStr::new("fixture")],
    );
    let json = json_output(&output);

    assert_eq!(json["schema_version"], 1);
    assert_eq!(json["installations"].as_array().unwrap().len(), 1);
    assert_path_eq(
        &PathBuf::from(json["installations"][0]["executable"].as_str().unwrap()),
        &executable,
    );
}

#[test]
fn all_reports_shadowed_commands_in_path_order() {
    let fs = TestFs::new();
    let first = fs.executable(Path::new("first").join(fixture_name("shadowed")));
    let second = fs.executable(Path::new("second").join(fixture_name("shadowed")));
    let path = [first.parent().unwrap(), second.parent().unwrap()];

    let default = json_output(&run_how(
        path,
        &[OsStr::new("--json"), OsStr::new("shadowed")],
    ));
    assert_eq!(default["installations"].as_array().unwrap().len(), 1);
    assert_path_eq(
        &PathBuf::from(default["installations"][0]["executable"].as_str().unwrap()),
        &first,
    );

    let all = json_output(&run_how(
        path,
        &[
            OsStr::new("--json"),
            OsStr::new("--all"),
            OsStr::new("shadowed"),
        ],
    ));
    let executables: Vec<_> = all["installations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|installation| {
            PathBuf::from(
                installation["executable"]
                    .as_str()
                    .expect("executable string"),
            )
        })
        .collect();
    assert_eq!(executables.len(), 2);
    assert_path_eq(&executables[0], &first);
    assert_path_eq(&executables[1], &second);
}

#[test]
fn missing_command_has_a_nonzero_status_and_no_json_payload() {
    let fs = TestFs::new();
    let empty = fs.root.join("empty");
    fs::create_dir(&empty).unwrap();
    let output = run_how(
        [&empty],
        &[OsStr::new("--json"), OsStr::new("definitely-missing")],
    );

    assert_eq!(output.status.code(), Some(1));
    assert!(
        output.stdout.is_empty(),
        "errors must not emit partial JSON"
    );
    let stderr = String::from_utf8(output.stderr).expect("stderr is UTF-8");
    assert!(stderr.starts_with("how: "), "{stderr}");
    assert!(stderr.contains("definitely-missing"), "{stderr}");
    assert!(stderr.contains("was not found in PATH"), "{stderr}");
}

#[cfg(target_os = "linux")]
#[test]
fn detects_a_linux_snap_path() {
    let fs = TestFs::new();
    let executable = fs.executable(Path::new("snap/bin").join(fixture_name("ripgrep")));
    let json = json_output(&run_how(
        [executable.parent().unwrap()],
        &[OsStr::new("--json"), OsStr::new("ripgrep")],
    ));

    assert_eq!(json["installations"][0]["manager"], "Snap");
    assert_eq!(json["installations"][0]["package"], "ripgrep");
    assert_eq!(json["installations"][0]["confidence"], "high");
}

#[cfg(target_os = "macos")]
#[test]
fn detects_a_macos_homebrew_cellar_path() {
    let fs = TestFs::new();
    let executable =
        fs.executable(Path::new("homebrew/Cellar/ripgrep/14.1.1/bin").join(fixture_name("rg")));
    let json = json_output(&run_how(
        [executable.parent().unwrap()],
        &[OsStr::new("--json"), OsStr::new("rg")],
    ));

    assert_eq!(json["installations"][0]["manager"], "Homebrew");
    assert_eq!(json["installations"][0]["package"], "ripgrep");
    assert_eq!(json["installations"][0]["version"], "14.1.1");
}

#[cfg(windows)]
#[test]
fn detects_a_windows_scoop_path() {
    let fs = TestFs::new();
    let scoop = fs.root.join("scoop");
    let executable =
        fs.executable(Path::new("scoop/apps/ripgrep/current").join(fixture_name("rg")));
    let path = std::env::join_paths([executable.parent().unwrap()])
        .expect("fixture PATH is representable");
    let mut command = Command::new(how_binary());
    command
        .env_clear()
        .env("PATH", path)
        .env("PATHEXT", ".EXE")
        .env("SCOOP", scoop)
        .args(["--json", "rg"]);
    for name in ["SystemRoot", "WINDIR"] {
        if let Some(value) = std::env::var_os(name) {
            command.env(name, value);
        }
    }
    let json = json_output(&command.output().expect("execute how binary"));

    assert_eq!(json["installations"][0]["manager"], "Scoop");
    assert_eq!(json["installations"][0]["package"], "ripgrep");
    assert_eq!(json["installations"][0]["confidence"], "high");
}
