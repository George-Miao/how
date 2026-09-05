mod bash;
mod fish;
mod nushell;
mod powershell;
mod tcsh;
mod zsh;

use std::env;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

use crate::command_probe::{self, CommandProbe, CommandSpec};
use crate::error::Error;

const COMMAND_ENV: &str = "HOW_ALIAS_COMMAND";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Expansion {
    pub name: OsString,
    pub value: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum AliasResolution {
    #[default]
    Auto,
    Disabled,
    Explicit(OsString),
}

trait Shell: Send + Sync {
    fn names(&self) -> &'static [&'static str];
    fn query(&self, probe: &dyn CommandProbe, program: &OsStr, command: &OsStr) -> Option<String>;

    fn validation_arguments(&self) -> &'static [&'static str] {
        &["-c", ""]
    }
}

static SHELLS: &[&dyn Shell] = &[
    &zsh::SHELL,
    &bash::SHELL,
    &fish::SHELL,
    &nushell::SHELL,
    &powershell::SHELL,
    &tcsh::SHELL,
];

pub fn validate(configuration: &AliasResolution) -> Result<(), Error> {
    let Some((shell, program, explicit)) = select_shell(configuration)? else {
        return Ok(());
    };
    validate_selected(shell, &program, explicit)
}

pub fn expand(
    command: &OsStr,
    configuration: &AliasResolution,
) -> Result<(OsString, Vec<Expansion>), Error> {
    let Some((shell, program, explicit)) = select_shell(configuration)? else {
        return Ok((command.to_os_string(), Vec::new()));
    };
    validate_selected(shell, &program, explicit)?;
    let probe = command_probe::system();

    Ok(expand_with(command, |command| {
        shell.query(probe, &program, command)
    }))
}

fn validate_selected(
    shell: &'static dyn Shell,
    program: &OsStr,
    explicit: bool,
) -> Result<(), Error> {
    if explicit
        && command_probe::system()
            .output(CommandSpec::new(program).args(shell.validation_arguments()))
            .is_none()
    {
        return Err(Error::ShellUnavailable {
            shell: program.to_os_string(),
        });
    }
    Ok(())
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

fn select_shell(
    configuration: &AliasResolution,
) -> Result<Option<(&'static dyn Shell, OsString, bool)>, Error> {
    let (configured, explicit) = match configuration {
        AliasResolution::Auto => {
            // SHELL is authoritative on every platform. In particular, Windows
            // does not imply PowerShell when SHELL is absent.
            let Some(configured) = env::var_os("SHELL") else {
                return Ok(None);
            };
            (configured, false)
        }
        AliasResolution::Disabled => return Ok(None),
        AliasResolution::Explicit(configured) => (configured.clone(), true),
    };
    let path = Path::new(&configured);
    let name = path.file_stem().map(|name| name.to_string_lossy());
    let shell = name.and_then(|name| {
        SHELLS.iter().copied().find(|shell| {
            shell
                .names()
                .iter()
                .any(|candidate| name.eq_ignore_ascii_case(candidate))
        })
    });
    let Some(shell) = shell else {
        return if explicit {
            Err(Error::UnsupportedShell { shell: configured })
        } else {
            Ok(None)
        };
    };

    let has_path = path.is_absolute() || path.components().count() > 1;
    if explicit && has_path && !path.is_file() {
        return Err(Error::InvalidShellPath {
            path: PathBuf::from(configured),
        });
    }
    let program = if path.is_file() || !has_path {
        configured
    } else {
        path.file_name()
            .map(OsString::from)
            .expect("supported shell has a file name")
    };
    Ok(Some((shell, program, explicit)))
}

fn query_output(
    probe: &dyn CommandProbe,
    program: &OsStr,
    command: &OsStr,
    script: &str,
) -> Option<std::sync::Arc<[u8]>> {
    probe.output(
        CommandSpec::new(program)
            .args(["-ic", script])
            .env(COMMAND_ENV, command),
    )
}

fn framed_value(output: &[u8]) -> Option<String> {
    let end = output.iter().rposition(|byte| *byte == 0)?;
    let start = output[..end].iter().rposition(|byte| *byte == 0)?;
    let value = String::from_utf8_lossy(&output[start + 1..end]).into_owned();
    (!value.is_empty()).then_some(value)
}

fn first_word(value: &str) -> Option<OsString> {
    let words = shell_words(value);
    wrapped_command(&words)
}

fn wrapped_command(words: &[OsString]) -> Option<OsString> {
    let mut index = 0;
    loop {
        let word = words.get(index)?.to_str()?;
        match word {
            "command" => {
                index += 1;
                if words
                    .get(index)
                    .is_some_and(|option| option.as_os_str() == OsStr::new("--"))
                {
                    index += 1;
                }
                if words
                    .get(index)
                    .and_then(|word| word.to_str())
                    .is_some_and(|option| option.starts_with('-'))
                {
                    return None;
                }
            }
            "env" => {
                index += 1;
                while let Some(argument) = words.get(index).and_then(|word| word.to_str()) {
                    match argument {
                        "--" => index += 1,
                        "-u" | "--unset" => {
                            let name = words.get(index + 1)?.to_str()?;
                            if name.eq_ignore_ascii_case("PATH") {
                                return None;
                            }
                            index += 2;
                        }
                        option if option.starts_with("--unset=") => {
                            let name = option
                                .strip_prefix("--unset=")
                                .expect("prefix matched above");
                            if name.eq_ignore_ascii_case("PATH") {
                                return None;
                            }
                            index += 1;
                        }
                        option if option.starts_with('-') => return None,
                        assignment if is_environment_assignment(assignment) => {
                            if assignment
                                .split_once('=')
                                .is_some_and(|(name, _)| name.eq_ignore_ascii_case("PATH"))
                            {
                                return None;
                            }
                            index += 1;
                        }
                        _ => break,
                    }
                }
            }
            _ => return Some(words[index].clone()),
        }
    }
}

fn is_environment_assignment(value: &str) -> bool {
    let Some((name, _)) = value.split_once('=') else {
        return false;
    };
    let mut characters = name.chars();
    characters
        .next()
        .is_some_and(|character| character == '_' || character.is_ascii_alphabetic())
        && characters.all(|character| character == '_' || character.is_ascii_alphanumeric())
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
    fn stops_alias_cycles_at_the_repeated_command() {
        let (target, expansions) = expand_with(OsStr::new("a"), |command| match command.to_str() {
            Some("a") => Some("b --first".into()),
            Some("b") => Some("a --second".into()),
            _ => None,
        });

        assert_eq!(target, "a");
        assert_eq!(
            expansions
                .iter()
                .map(|expansion| expansion.name.as_os_str())
                .collect::<Vec<_>>(),
            [OsStr::new("a"), OsStr::new("b")]
        );
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
    fn unwraps_safe_env_and_command_wrappers() {
        assert_eq!(
            first_word("env -u DEBUG MODE=fast -- command -- 'my tool' --flag"),
            Some(OsString::from("my tool"))
        );
        assert_eq!(
            first_word("command -- /opt/tools/rg --hidden"),
            Some(OsString::from("/opt/tools/rg"))
        );
    }

    #[test]
    fn leaves_unsafe_wrapper_forms_unresolved() {
        assert_eq!(first_word("env -S 'tool --flag'"), None);
        assert_eq!(first_word("env PATH=/tmp tool"), None);
        assert_eq!(first_word("command -p tool"), None);
        assert_eq!(first_word("command -v tool"), None);
    }

    #[test]
    fn splits_shell_quoted_words() {
        assert_eq!(
            shell_words("alias ll 'eza --long'"),
            ["alias", "ll", "eza --long"].map(OsString::from)
        );
    }
}
