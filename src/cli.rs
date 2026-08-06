use std::ffi::OsString;
use std::fmt::Write;

use clap::Parser;

use crate::detect::{self, Installation};
use crate::error::Error;

#[derive(Debug, Parser, PartialEq, Eq)]
#[command(
    name = "how",
    version,
    about = "How a command was installed?",
    after_help = "Examples:\n  how rg\n  how --all python\n  how --json /opt/homebrew/bin/rg"
)]
struct Cli {
    /// Inspect every matching executable in PATH
    #[arg(short = 'a', long)]
    all: bool,

    /// Print machine-readable JSON
    #[arg(long)]
    json: bool,

    /// Command name or executable path to inspect
    #[arg(value_name = "COMMAND")]
    command: OsString,
}

pub fn run() -> Result<(), Error> {
    let options = Cli::parse();
    let installations = detect::inspect(&options.command, options.all)?;
    if options.json {
        println!("{}", render_json(&installations));
    } else {
        print!("{}", render_human(&installations));
    }
    Ok(())
}

fn render_human(installations: &[Installation]) -> String {
    let mut output = String::new();
    for (index, installation) in installations.iter().enumerate() {
        if index > 0 {
            output.push('\n');
        }
        let _ = writeln!(output, "{}", installation.executable.display());
        if installation.executable != installation.resolved {
            let _ = writeln!(output, "  resolves to  {}", installation.resolved.display());
        }
        let _ = writeln!(
            output,
            "  installed by {} ({})",
            installation.manager, installation.confidence
        );
        if let Some(package) = &installation.package {
            let _ = writeln!(output, "  package      {package}");
        }
        for evidence in &installation.evidence {
            let _ = writeln!(
                output,
                "  evidence     {}: {}",
                evidence.kind, evidence.detail
            );
        }
    }
    output
}

fn render_json(installations: &[Installation]) -> String {
    let mut output = String::from("[");
    for (index, installation) in installations.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"executable\":");
        push_json_string(&mut output, &installation.executable.to_string_lossy());
        output.push_str(",\"resolved\":");
        push_json_string(&mut output, &installation.resolved.to_string_lossy());
        output.push_str(",\"manager\":");
        push_json_string(&mut output, installation.manager);
        output.push_str(",\"package\":");
        match &installation.package {
            Some(package) => push_json_string(&mut output, package),
            None => output.push_str("null"),
        }
        output.push_str(",\"confidence\":");
        push_json_string(&mut output, installation.confidence.as_str());
        output.push_str(",\"evidence\":[");
        for (evidence_index, evidence) in installation.evidence.iter().enumerate() {
            if evidence_index > 0 {
                output.push(',');
            }
            output.push_str("{\"kind\":");
            push_json_string(&mut output, evidence.kind);
            output.push_str(",\"detail\":");
            push_json_string(&mut output, &evidence.detail);
            output.push('}');
        }
        output.push_str("]}");
    }
    output.push(']');
    output
}

fn push_json_string(output: &mut String, value: &str) {
    output.push('"');
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            character if character.is_control() => {
                let _ = write!(output, "\\u{:04x}", character as u32);
            }
            character => output.push(character),
        }
    }
    output.push('"');
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::provider::{Confidence, Evidence};

    #[test]
    fn clap_parses_options() {
        let options = Cli::try_parse_from(["how", "--json", "--all", "rg"]).unwrap();
        assert_eq!(
            options,
            Cli {
                command: OsString::from("rg"),
                all: true,
                json: true,
            }
        );
    }

    #[test]
    fn json_output_escapes_values() {
        let installation = Installation {
            executable: PathBuf::from("/tmp/a\"b"),
            resolved: PathBuf::from("/tmp/a\"b"),
            manager: "unknown",
            package: None,
            confidence: Confidence::Low,
            evidence: vec![Evidence {
                kind: "path convention",
                detail: "line\none".into(),
            }],
        };

        let json = render_json(&[installation]);
        assert!(json.contains("a\\\"b"));
        assert!(json.contains("\"confidence\":\"low\""));
        assert_eq!(render_json(&[]), "[]");
    }
}
