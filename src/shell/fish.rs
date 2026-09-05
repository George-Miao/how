use std::ffi::OsStr;

use super::{CommandProbe, Shell, framed_value, query_output, shell_words};

const QUERY: &str = "printf '\\0'; alias; printf '\\0'";

pub(super) struct Fish;
pub(super) static SHELL: Fish = Fish;

impl Shell for Fish {
    fn names(&self) -> &'static [&'static str] {
        &["fish"]
    }

    fn query(&self, probe: &dyn CommandProbe, program: &OsStr, command: &OsStr) -> Option<String> {
        let output = query_output(probe, program, command, QUERY)?;
        parse(&output, command)
    }
}

fn parse(output: &[u8], command: &OsStr) -> Option<String> {
    framed_value(output)?.lines().find_map(|line| {
        let words = shell_words(line);
        if words.first()? == "alias" && words.get(1)? == command {
            Some(words.get(2)?.to_string_lossy().into_owned())
        } else {
            None
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_requested_alias_inside_profile_noise() {
        let output = b"profile banner\n\0alias g 'git'\nalias ll 'eza --long'\n\0prompt text\n";
        assert_eq!(parse(output, OsStr::new("ll")), Some("eza --long".into()));
        assert_eq!(parse(output, OsStr::new("missing")), None);
    }
}
