use std::ffi::OsString;
use std::fmt::Write;

use clap::builder::styling;
use clap::{ColorChoice, Parser};
use owo_colors::Stream::Stdout;
use owo_colors::{OwoColorize, Style};
use serde::Serialize;

use crate::detect::{self, CandidateExplanation, Installation};
use crate::error::Error;
use crate::provider::Provenance;

const CLAP_STYLES: styling::Styles = styling::Styles::styled()
    .header(styling::AnsiColor::Cyan.on_default().bold())
    .usage(styling::AnsiColor::Cyan.on_default().bold())
    .literal(styling::AnsiColor::Green.on_default().bold())
    .placeholder(styling::AnsiColor::BrightCyan.on_default());

#[derive(Debug, Parser, PartialEq, Eq)]
#[command(
    name = "how",
    version,
    color = ColorChoice::Auto,
    styles = CLAP_STYLES,
    about = "How was a command installed?",
    after_help = "Examples:\n  how rg\n  how --all python\n  how --explain rg\n  how --json /opt/homebrew/bin/rg"
)]
struct Cli {
    /// Inspect every matching executable in PATH
    #[arg(short = 'a', long)]
    all: bool,

    /// Print machine-readable JSON
    #[arg(long)]
    json: bool,
    /// Explain candidate selection
    #[arg(long, conflicts_with = "json")]
    explain: bool,

    /// Command name or executable path to inspect
    #[arg(value_name = "COMMAND")]
    command: OsString,
}

pub fn run() -> Result<(), Error> {
    let options = Cli::parse();
    let installations = detect::inspect(&options.command, options.all, options.explain)?;
    if options.json {
        println!("{}", render_json(&installations));
    } else {
        print!("{}", render_human(&installations, options.explain));
    }
    Ok(())
}

fn render_human(installations: &[Installation], explain: bool) -> String {
    let mut output = String::new();
    for (index, installation) in installations.iter().enumerate() {
        if index > 0 {
            output.push('\n');
        }
        let _ = writeln!(
            output,
            "{}",
            installation
                .executable
                .display()
                .if_supports_color(Stdout, |path| {
                    Style::new().bright_cyan().bold().style(path)
                })
        );
        if installation.executable != installation.resolved {
            let _ = writeln!(
                output,
                "  → {}",
                installation
                    .resolved
                    .display()
                    .if_supports_color(Stdout, |path| path.cyan())
            );
        }
        let _ = writeln!(
            output,
            "  manager      {}",
            installation.manager.if_supports_color(Stdout, |manager| {
                Style::new().bright_green().bold().style(manager)
            })
        );
        render_provenance(&mut output, "  ", &installation.provenance);

        let confidence = installation.confidence.as_str();
        let confidence_style = match installation.confidence {
            crate::provider::Confidence::High => Style::new().bright_green().bold(),
            crate::provider::Confidence::Medium => Style::new().bright_yellow().bold(),
            crate::provider::Confidence::Low => Style::new().bright_red().bold(),
        };
        let _ = writeln!(
            output,
            "  confidence   {}",
            confidence.if_supports_color(Stdout, |value| value.style(confidence_style))
        );

        let _ = writeln!(
            output,
            "  {}",
            "evidence".if_supports_color(Stdout, |label| { label.bold() })
        );
        for evidence in &installation.evidence {
            let _ = writeln!(
                output,
                "    • {}: {}",
                evidence
                    .kind
                    .if_supports_color(Stdout, |kind| kind.bright_yellow()),
                evidence
                    .detail
                    .if_supports_color(Stdout, |detail| detail.dimmed())
            );
        }
        if explain && let Some(arbitration) = &installation.arbitration {
            let _ = writeln!(
                output,
                "  {}",
                "arbitration".if_supports_color(Stdout, |label| label.bold())
            );
            render_candidate(&mut output, "selected", &arbitration.selected);
            for rejected in &arbitration.rejected {
                render_candidate(&mut output, "rejected", rejected);
            }
        }
    }
    output
}

fn render_candidate(output: &mut String, disposition: &str, candidate: &CandidateExplanation) {
    let _ = writeln!(
        output,
        "    {disposition} {} ({}; {}; {})",
        candidate.manager,
        candidate.confidence.as_str(),
        candidate.phase,
        candidate.mechanism
    );
    render_provenance(output, "      ", &candidate.provenance);
    let _ = writeln!(output, "      evidence: {}", candidate.detail);
    let _ = writeln!(output, "      reason: {}", candidate.reason);
}

fn render_provenance(output: &mut String, indent: &str, provenance: &Provenance) {
    for (label, value) in [
        ("package", &provenance.package),
        ("version", &provenance.version),
        ("environment", &provenance.environment),
        ("toolchain", &provenance.toolchain),
        ("derivation", &provenance.derivation),
    ] {
        if let Some(value) = value {
            let _ = writeln!(
                output,
                "{indent}{label:<12}{}",
                value.if_supports_color(Stdout, |value| value.bright_magenta())
            );
        }
    }
}

const JSON_SCHEMA_VERSION: u8 = 1;

#[derive(Serialize)]
struct JsonOutput<'a> {
    schema_version: u8,
    installations: &'a [Installation],
}

fn render_json(installations: &[Installation]) -> String {
    serde_json::to_string(&JsonOutput {
        schema_version: JSON_SCHEMA_VERSION,
        installations,
    })
    .expect("JSON output contains only serializable values")
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use clap::CommandFactory;

    use super::*;
    use crate::provider::{Confidence, Evidence, Provenance};

    #[test]
    fn clap_parses_options() {
        let options = Cli::try_parse_from(["how", "--json", "--all", "rg"]).unwrap();
        assert_eq!(
            options,
            Cli {
                command: OsString::from("rg"),
                all: true,
                json: true,
                explain: false,
            }
        );
    }
    #[test]
    fn clap_rejects_explain_with_json() {
        let error = Cli::try_parse_from(["how", "--explain", "--json", "rg"])
            .expect_err("options conflict");

        assert_eq!(error.kind(), clap::error::ErrorKind::ArgumentConflict);
        let message = error.to_string();
        assert!(message.contains("--explain"), "{message}");
        assert!(message.contains("--json"), "{message}");
    }

    #[test]
    fn clap_help_has_color_when_enabled() {
        let help = Cli::command()
            .color(ColorChoice::Always)
            .render_help()
            .ansi()
            .to_string();
        assert!(help.contains("\u{1b}["));
    }
    #[test]
    fn explain_output_lists_selected_and_rejected_candidates() {
        let installation = Installation {
            executable: PathBuf::from("/nix/store/abc-ripgrep/bin/rg"),
            resolved: PathBuf::from("/nix/store/abc-ripgrep/bin/rg"),
            manager: "Nix",
            provenance: Provenance {
                derivation: Some("ripgrep-14.1.1".into()),
                ..Provenance::default()
            },
            confidence: Confidence::High,
            evidence: vec![Evidence {
                kind: "path convention",
                detail: "target lives in /nix/store".into(),
            }],
            arbitration: Some(crate::detect::ArbitrationExplanation {
                selected: CandidateExplanation {
                    manager: "Nix",
                    provenance: Provenance {
                        derivation: Some("ripgrep-14.1.1".into()),
                        ..Provenance::default()
                    },
                    confidence: Confidence::High,
                    phase: "cheap path",
                    mechanism: "path convention",
                    detail: "target lives in /nix/store".into(),
                    reason: "selected by confidence-first policy".into(),
                },
                rejected: vec![CandidateExplanation {
                    manager: "Cargo",
                    provenance: Provenance {
                        package: Some("rg".into()),
                        ..Provenance::default()
                    },
                    confidence: Confidence::Medium,
                    phase: "cheap path",
                    mechanism: "path convention",
                    detail: "executable is in Cargo install root".into(),
                    reason: "lower confidence than selected Nix".into(),
                }],
            }),
        };

        let output = render_human(std::slice::from_ref(&installation), true);
        assert!(output.contains("arbitration"), "{output}");
        assert!(output.contains("selected Nix"), "{output}");
        assert!(output.contains("rejected Cargo"), "{output}");
        assert!(output.contains("lower confidence"), "{output}");
        assert!(output.contains("derivation  ripgrep-14.1.1"), "{output}");
        assert!(!render_human(&[installation], false).contains("arbitration"));
    }

    #[test]
    fn json_output_has_a_versioned_stable_contract() {
        let installation = Installation {
            executable: PathBuf::from("/tmp/a\"b"),
            resolved: PathBuf::from("/tmp/a\"b"),
            manager: "unknown",
            provenance: Provenance {
                package: Some("ripgrep".into()),
                version: Some("14.1.1".into()),
                environment: Some("dev".into()),
                toolchain: Some("rust".into()),
                derivation: Some("ripgrep-14.1.1".into()),
            },
            confidence: Confidence::Low,
            evidence: vec![Evidence {
                kind: "path convention",
                detail: "line\none".into(),
            }],
            arbitration: None,
        };

        assert_eq!(
            render_json(&[installation]),
            concat!(
                "{\"schema_version\":1,\"installations\":[",
                "{\"executable\":\"/tmp/a\\\"b\",\"resolved\":\"/tmp/a\\\"b\",",
                "\"manager\":\"unknown\",\"package\":\"ripgrep\",\"version\":\"14.1.1\",",
                "\"environment\":\"dev\",\"toolchain\":\"rust\",",
                "\"derivation\":\"ripgrep-14.1.1\",\"confidence\":\"low\",",
                "\"evidence\":[{\"kind\":\"path convention\",\"detail\":\"line\\none\"}]}]}"
            )
        );
        assert_eq!(
            render_json(&[]),
            "{\"schema_version\":1,\"installations\":[]}"
        );
    }

    #[test]
    fn json_output_keeps_nullable_provenance_fields() {
        let installation = Installation {
            executable: PathBuf::from("/usr/bin/tool"),
            resolved: PathBuf::from("/usr/bin/tool"),
            manager: "unknown",
            provenance: Provenance::default(),
            confidence: Confidence::Low,
            evidence: Vec::new(),
            arbitration: None,
        };

        assert_eq!(
            render_json(&[installation]),
            concat!(
                "{\"schema_version\":1,\"installations\":[",
                "{\"executable\":\"/usr/bin/tool\",\"resolved\":\"/usr/bin/tool\",",
                "\"manager\":\"unknown\",\"package\":null,\"version\":null,",
                "\"environment\":null,\"toolchain\":null,\"derivation\":null,",
                "\"confidence\":\"low\",\"evidence\":[]}]}"
            )
        );
    }

    #[cfg(unix)]
    #[test]
    fn json_output_preserves_lossy_non_utf8_path_compatibility() {
        use std::os::unix::ffi::OsStringExt;

        let path = PathBuf::from(OsString::from_vec(b"/tmp/a\xffb".to_vec()));
        let installation = Installation {
            executable: path.clone(),
            resolved: path,
            manager: "unknown",
            provenance: Provenance::default(),
            confidence: Confidence::Low,
            evidence: Vec::new(),
            arbitration: None,
        };

        let json = render_json(&[installation]);
        assert!(json.contains("/tmp/a�b"), "{json}");
    }
}
