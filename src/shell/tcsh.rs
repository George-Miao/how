use std::ffi::OsStr;

use super::{Shell, query_output};

const QUERY: &str = "alias \"$HOW_ALIAS_COMMAND\"";

pub(super) struct Tcsh;
pub(super) static SHELL: Tcsh = Tcsh;

impl Shell for Tcsh {
    fn names(&self) -> &'static [&'static str] {
        &["tcsh", "csh"]
    }

    fn query(&self, program: &OsStr, command: &OsStr) -> Option<String> {
        let output = query_output(program, command, QUERY)?;
        parse(&output)
    }
}

fn parse(output: &[u8]) -> Option<String> {
    let value = String::from_utf8_lossy(output)
        .lines()
        .next_back()?
        .trim()
        .to_owned();
    (!value.is_empty()).then_some(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uses_last_line_after_startup_output() {
        assert_eq!(parse(b"startup\nls -la\n"), Some("ls -la".into()));
    }
}
