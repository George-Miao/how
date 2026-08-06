use std::ffi::OsStr;

use super::{Shell, first_word, framed_value, query_output};

const QUERY: &str = "printf '\\0'; alias -- \"$HOW_ALIAS_COMMAND\" 2>/dev/null; printf '\\0'";

pub(super) struct Bash;
pub(super) static SHELL: Bash = Bash;

impl Shell for Bash {
    fn names(&self) -> &'static [&'static str] {
        &["bash"]
    }

    fn query(&self, program: &OsStr, command: &OsStr) -> Option<String> {
        let output = query_output(program, command, QUERY)?;
        parse(&output)
    }
}

fn parse(output: &[u8]) -> Option<String> {
    let definition = framed_value(output)?;
    let encoded = definition.trim().strip_prefix("alias ")?.split_once('=')?.1;
    first_word(encoded)?.into_string().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_bash_3_alias_output() {
        assert_eq!(parse(b"\0alias ll='ls -la'\n\0"), Some("ls -la".into()));
        assert_eq!(
            parse(b"\0alias p='printf '\\''%s\\n'\\'''\n\0"),
            Some("printf '%s\\n'".into())
        );
    }
}
