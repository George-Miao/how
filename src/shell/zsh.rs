use std::ffi::OsStr;

use super::{CommandProbe, Shell, framed_value, query_output};

const QUERY: &str = "builtin print -rn -- $'\\0'${aliases[$HOW_ALIAS_COMMAND]-}$'\\0'";

pub(super) struct Zsh;
pub(super) static SHELL: Zsh = Zsh;

impl Shell for Zsh {
    fn names(&self) -> &'static [&'static str] {
        &["zsh"]
    }

    fn query(&self, probe: &dyn CommandProbe, program: &OsStr, command: &OsStr) -> Option<String> {
        let output = query_output(probe, program, command, QUERY)?;
        framed_value(&output)
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;
    use std::process::{Command, Stdio};

    use super::*;

    #[test]
    fn ignores_shell_startup_output_around_framed_alias() {
        assert_eq!(
            framed_value(b"startup message\n\0eza --long\0"),
            Some("eza --long".into())
        );
    }

    #[test]
    fn query_ignores_aliases_for_print() {
        assert!(QUERY.starts_with("builtin print "));
        let child = Command::new("zsh")
            .arg("-f")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .env("HOW_ALIAS_COMMAND", "ll")
            .spawn();
        let Ok(mut child) = child else {
            return;
        };
        writeln!(
            child.stdin.take().unwrap(),
            "alias ll='eza -l'\nalias print=false\n{QUERY}"
        )
        .unwrap();
        let output = child.wait_with_output().unwrap();

        assert_eq!(framed_value(&output.stdout), Some("eza -l".into()));
    }
}
