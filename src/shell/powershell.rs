use std::ffi::OsStr;

use super::{CommandProbe, CommandSpec, Shell, framed_value};

const QUERY: &str = "$a = Microsoft.PowerShell.Utility\\Get-Alias -Name $env:HOW_ALIAS_COMMAND \
                     -ErrorAction SilentlyContinue; if ($null -ne $a) { \
                     [Console]::Out.Write(\"`0$($a.Definition)`0\") }";

pub(super) struct PowerShell;
pub(super) static SHELL: PowerShell = PowerShell;

impl Shell for PowerShell {
    fn names(&self) -> &'static [&'static str] {
        &["pwsh", "powershell"]
    }

    fn query(&self, probe: &dyn CommandProbe, program: &OsStr, command: &OsStr) -> Option<String> {
        let output = probe.output(
            CommandSpec::new(program)
                .args(["-NoLogo", "-NonInteractive", "-Command", QUERY])
                .env("HOW_ALIAS_COMMAND", command),
        )?;
        framed_value(&output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_framed_definition() {
        assert_eq!(
            framed_value(b"profile output\n\0Get-ChildItem\0"),
            Some("Get-ChildItem".into())
        );
    }
}
