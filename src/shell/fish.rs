use std::ffi::OsStr;

use super::{Shell, query_output, shell_words};

const QUERY: &str = "alias";

pub(super) struct Fish;
pub(super) static SHELL: Fish = Fish;

impl Shell for Fish {
    fn names(&self) -> &'static [&'static str] {
        &["fish"]
    }

    fn query(&self, program: &OsStr, command: &OsStr) -> Option<String> {
        let output = query_output(program, command, QUERY)?;
        parse(&output, command)
    }
}

fn parse(output: &[u8], command: &OsStr) -> Option<String> {
    String::from_utf8_lossy(output).lines().find_map(|line| {
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
    fn finds_requested_alias_in_listing() {
        let output = b"alias g 'git'\nalias ll 'eza --long'\n";
        assert_eq!(parse(output, OsStr::new("ll")), Some("eza --long".into()));
        assert_eq!(parse(output, OsStr::new("missing")), None);
    }
}
