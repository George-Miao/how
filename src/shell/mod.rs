mod bash;
mod fish;
mod nushell;
mod powershell;
mod tcsh;
mod zsh;

use std::env;
use std::ffi::{OsStr, OsString};
use std::path::Path;
use std::process::Command;

const COMMAND_ENV: &str = "HOW_ALIAS_COMMAND";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Expansion {
    pub name: OsString,
    pub value: String,
}

trait Shell: Send + Sync {
    fn names(&self) -> &'static [&'static str];
    fn query(&self, program: &OsStr, command: &OsStr) -> Option<String>;
}

static SHELLS: &[&dyn Shell] = &[
    &zsh::SHELL,
    &bash::SHELL,
    &fish::SHELL,
    &nushell::SHELL,
    &powershell::SHELL,
    &tcsh::SHELL,
];

pub fn expand(command: &OsStr) -> (OsString, Vec<Expansion>) {
    expand_with(command, query)
}

fn expand_with(
    command: &OsStr,
    mut query: impl FnMut(&OsStr) -> Option<String>,
) -> (OsString, Vec<Expansion>) {
    let mut current = command.to_os_string();
    let mut seen = vec![current.clone()];
    let mut expansions = Vec::new();

    while let Some(value) = query(&current) {
        let Some(target) = first_word(&value) else {
            break;
        };
        expansions.push(Expansion {
            name: current,
            value,
        });
        current = target;
        if seen.contains(&current) {
            break;
        }
        seen.push(current.clone());
    }

    (current, expansions)
}

fn query(command: &OsStr) -> Option<String> {
    let configured = env::var_os("SHELL")?;
    let path = Path::new(&configured);
    let name = path.file_stem()?.to_string_lossy();
    let shell = SHELLS.iter().find(|shell| {
        shell
            .names()
            .iter()
            .any(|candidate| name.eq_ignore_ascii_case(candidate))
    })?;
    let program = path
        .is_file()
        .then(|| configured.clone())
        .or_else(|| path.file_name().map(OsString::from))?;
    shell.query(&program, command)
}

fn query_output(program: &OsStr, command: &OsStr, script: &str) -> Option<Vec<u8>> {
    Command::new(program)
        .args(["-ic", script])
        .env(COMMAND_ENV, command)
        .output()
        .ok()
        .map(|output| output.stdout)
}

fn framed_value(output: &[u8]) -> Option<String> {
    let end = output.iter().rposition(|byte| *byte == 0)?;
    let start = output[..end].iter().rposition(|byte| *byte == 0)?;
    let value = String::from_utf8_lossy(&output[start + 1..end]).into_owned();
    (!value.is_empty()).then_some(value)
}

fn first_word(value: &str) -> Option<OsString> {
    shell_words(value).into_iter().next()
}

fn shell_words(value: &str) -> Vec<OsString> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut characters = value.chars();
    let mut quote = None;
    let mut started = false;

    while let Some(character) = characters.next() {
        match (quote, character) {
            (None, character) if character.is_whitespace() => {
                if started {
                    words.push(std::mem::take(&mut word).into());
                    started = false;
                }
            }
            (None, '\'' | '"') => {
                quote = Some(character);
                started = true;
            }
            (Some(active), character) if character == active => quote = None,
            (None | Some('"'), '\\') => {
                if let Some(escaped) = characters.next() {
                    word.push(escaped);
                    started = true;
                }
            }
            _ => {
                word.push(character);
                started = true;
            }
        }
    }

    if started {
        words.push(word.into());
    }
    words
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_alias_chains_without_evaluating_arguments() {
        let (target, expansions) = expand_with(OsStr::new("l"), |command| match command.to_str() {
            Some("l") => Some("ll -a".into()),
            Some("ll") => Some("'/opt/local/bin/eza' --long".into()),
            _ => None,
        });

        assert_eq!(target, "/opt/local/bin/eza");
        assert_eq!(expansions.len(), 2);
        assert_eq!(expansions[0].value, "ll -a");
    }

    #[test]
    fn self_alias_resolves_to_the_path_command() {
        let (target, expansions) = expand_with(OsStr::new("rg"), |_| Some("rg --hidden".into()));

        assert_eq!(target, "rg");
        assert_eq!(expansions.len(), 1);
    }

    #[test]
    fn extracts_quoted_command_path() {
        assert_eq!(
            first_word("  '/Applications/My Tool/bin/tool' --flag"),
            Some(OsString::from("/Applications/My Tool/bin/tool"))
        );
    }

    #[test]
    fn splits_shell_quoted_words() {
        assert_eq!(
            shell_words("alias ll 'eza --long'"),
            ["alias", "ll", "eza --long"].map(OsString::from)
        );
    }
}
