use std::ffi::OsStr;

use super::{CommandProbe, Shell, framed_value, query_output};

const QUERY: &str = "printf '\\0'; alias \"$HOW_ALIAS_COMMAND\"; printf '\\0'";

pub(super) struct Tcsh;
pub(super) static SHELL: Tcsh = Tcsh;

impl Shell for Tcsh {
    fn names(&self) -> &'static [&'static str] {
        &["tcsh", "csh"]
    }

    fn query(&self, probe: &dyn CommandProbe, program: &OsStr, command: &OsStr) -> Option<String> {
        let output = query_output(probe, program, command, QUERY)?;
        parse(&output)
    }
}

fn parse(output: &[u8]) -> Option<String> {
    let value = framed_value(output)?.trim().to_owned();
    (!value.is_empty()).then_some(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_alias_inside_profile_noise() {
        assert_eq!(
            parse(b"profile banner\n\0ls -la\n\0prompt text\n"),
            Some("ls -la".into())
        );
    }
}
