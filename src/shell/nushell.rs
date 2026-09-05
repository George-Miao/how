use std::env;
use std::ffi::OsStr;
use std::path::PathBuf;

use super::{CommandProbe, CommandSpec, Shell};
use crate::util;

const QUERY: &str = "scope aliases | where name == $env.HOW_ALIAS_COMMAND | get -o 0.expansion | \
                     default '' | print -n";

pub(super) struct Nushell;
pub(super) static SHELL: Nushell = Nushell;

impl Shell for Nushell {
    fn names(&self) -> &'static [&'static str] {
        &["nu"]
    }

    fn query(&self, probe: &dyn CommandProbe, program: &OsStr, command: &OsStr) -> Option<String> {
        let config = config_file()?;
        let output = probe.output(
            CommandSpec::new(program)
                .args([
                    OsStr::new("--config"),
                    config.as_os_str(),
                    OsStr::new("-c"),
                    OsStr::new(QUERY),
                ])
                .env("HOW_ALIAS_COMMAND", command),
        )?;
        let value = String::from_utf8_lossy(&output).trim().to_owned();
        (!value.is_empty()).then_some(value)
    }
}

fn config_file() -> Option<PathBuf> {
    let configured = env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .map(|root| root.join("nushell/config.nu"));
    let platform = env::var_os("APPDATA")
        .map(PathBuf::from)
        .map(|root| root.join("nushell/config.nu"));
    let home = util::home_dir();
    let macos = home
        .as_ref()
        .map(|home| home.join("Library/Application Support/nushell/config.nu"));
    let conventional = home.map(|home| home.join(".config/nushell/config.nu"));
    configured
        .into_iter()
        .chain(platform)
        .chain(macos)
        .chain(conventional)
        .find(|path| path.is_file())
}
