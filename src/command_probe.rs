use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::io::Read;
use std::process::{Command as ProcessCommand, Stdio};
use std::sync::{Arc, LazyLock, Mutex, OnceLock, mpsc};
use std::time::{Duration, Instant};

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(2);
const POLL_INTERVAL: Duration = Duration::from_millis(5);

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CommandSpec {
    program: OsString,
    arguments: Vec<OsString>,
    environment: Vec<(OsString, OsString)>,
}

impl CommandSpec {
    pub fn new(program: impl AsRef<OsStr>) -> Self {
        Self {
            program: program.as_ref().to_owned(),
            arguments: Vec::new(),
            environment: Vec::new(),
        }
    }

    pub fn arg(mut self, argument: impl AsRef<OsStr>) -> Self {
        self.arguments.push(argument.as_ref().to_owned());
        self
    }

    pub fn args<I, S>(mut self, arguments: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.arguments.extend(
            arguments
                .into_iter()
                .map(|argument| argument.as_ref().to_owned()),
        );
        self
    }

    pub fn env(mut self, name: impl AsRef<OsStr>, value: impl AsRef<OsStr>) -> Self {
        self.environment
            .push((name.as_ref().to_owned(), value.as_ref().to_owned()));
        self
    }
}

pub trait CommandProbe: Send + Sync {
    fn output(&self, command: CommandSpec) -> Option<Arc<[u8]>>;
}

type CachedOutput = OnceLock<Option<Arc<[u8]>>>;

pub struct ProcessCommandProbe {
    timeout: Duration,
    cache: Mutex<HashMap<CommandSpec, Arc<CachedOutput>>>,
}

impl ProcessCommandProbe {
    fn new(timeout: Duration) -> Self {
        Self {
            timeout,
            cache: Mutex::new(HashMap::new()),
        }
    }

    fn output_uncached(&self, command: &CommandSpec) -> Option<Arc<[u8]>> {
        let mut child = ProcessCommand::new(&command.program)
            .args(&command.arguments)
            .envs(command.environment.iter().cloned())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .ok()?;
        let mut stdout = child.stdout.take()?;
        let (sender, receiver) = mpsc::sync_channel(1);
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            let output = stdout.read_to_end(&mut bytes).ok().map(|_| bytes);
            let _ = sender.send(output);
        });

        let deadline = Instant::now() + self.timeout;
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) => {
                    let remaining = deadline.saturating_duration_since(Instant::now());
                    if remaining.is_zero() {
                        let _ = child.kill();
                        let _ = child.wait();
                        return None;
                    }
                    std::thread::sleep(remaining.min(POLL_INTERVAL));
                }
                Err(_) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return None;
                }
            }
        };
        if !status.success() {
            return None;
        }

        receiver
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
            .ok()
            .flatten()
            .map(Arc::from)
    }
}

impl CommandProbe for ProcessCommandProbe {
    fn output(&self, command: CommandSpec) -> Option<Arc<[u8]>> {
        let cached = {
            let mut cache = self
                .cache
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            Arc::clone(
                cache
                    .entry(command.clone())
                    .or_insert_with(|| Arc::new(OnceLock::new())),
            )
        };
        cached
            .get_or_init(|| self.output_uncached(&command))
            .clone()
    }
}

pub fn system() -> &'static dyn CommandProbe {
    static PROBE: LazyLock<ProcessCommandProbe> =
        LazyLock::new(|| ProcessCommandProbe::new(DEFAULT_TIMEOUT));
    &*PROBE
}

#[cfg(test)]
mod tests {
    use std::fs::{self, OpenOptions};
    use std::io::Write;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{Duration, Instant};
    use std::{env, process};

    use super::*;

    static NEXT_FILE: AtomicU64 = AtomicU64::new(0);
    const HELPER_ENV: &str = "HOW_COMMAND_PROBE_TEST_HELPER";
    const COUNT_FILE_ENV: &str = "HOW_COMMAND_PROBE_TEST_COUNT_FILE";

    #[test]
    fn captures_successful_stdout() {
        let probe = ProcessCommandProbe::new(Duration::from_secs(1));

        let output = probe
            .output(helper_command("success"))
            .expect("successful probe output");

        assert!(String::from_utf8_lossy(&output).contains("probe-success"));
    }

    #[test]
    fn rejects_nonzero_exit_output() {
        let probe = ProcessCommandProbe::new(Duration::from_secs(1));

        assert!(probe.output(helper_command("nonzero")).is_none());
    }

    #[test]
    fn terminates_probe_at_timeout() {
        let probe = ProcessCommandProbe::new(Duration::from_millis(50));
        let started = Instant::now();

        assert!(probe.output(helper_command("timeout")).is_none());
        assert!(started.elapsed() < Duration::from_secs(1));
    }

    #[test]
    fn caches_identical_probe_results() {
        let count_file = env::temp_dir().join(format!(
            "how-command-probe-{}-{}",
            process::id(),
            NEXT_FILE.fetch_add(1, Ordering::Relaxed)
        ));
        let probe = ProcessCommandProbe::new(Duration::from_secs(1));
        let command = helper_command("success").env(COUNT_FILE_ENV, &count_file);

        probe.output(command.clone()).expect("first probe output");
        probe.output(command).expect("cached probe output");

        let invocations = fs::read_to_string(&count_file).expect("probe count file");
        assert_eq!(invocations.lines().count(), 1);
        fs::remove_file(count_file).expect("remove probe count file");
    }

    #[test]
    fn probe_process_helper() {
        match env::var(HELPER_ENV).as_deref() {
            Ok("success") => {
                if let Some(path) = env::var_os(COUNT_FILE_ENV) {
                    writeln!(
                        OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open(path)
                            .expect("open probe count file"),
                        "invoked"
                    )
                    .expect("record probe invocation");
                }
                print!("probe-success");
            }
            Ok("nonzero") => process::exit(17),
            Ok("timeout") => std::thread::sleep(Duration::from_secs(5)),
            _ => {}
        }
    }

    fn helper_command(scenario: &str) -> CommandSpec {
        CommandSpec::new(env::current_exe().expect("current test executable"))
            .args([
                "--exact",
                "command_probe::tests::probe_process_helper",
                "--nocapture",
            ])
            .env(HELPER_ENV, scenario)
    }
}
