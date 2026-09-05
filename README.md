# how

[![github]](https://github.com/George-Miao/how)
[![crates.io]](https://crates.io/crates/how)
[![docs.rs]](https://docs.rs/how)
[![build status]](https://github.com/George-Miao/how/actions?query=branch%3Amain)

[github]: https://img.shields.io/badge/github-George--Miao/how-8da0cb?labelColor=555555&logo=github&style=for-the-badge
[crates.io]: https://img.shields.io/crates/v/how.svg?color=fc8d62&logo=rust&style=for-the-badge
[docs.rs]: https://img.shields.io/badge/docs.rs-how-66c2a5?labelColor=555555&logo=docs.rs&style=for-the-badge
[build status]: https://img.shields.io/github/actions/workflow/status/George-Miao/how/ci.yml?branch=main&style=for-the-badge

`how` explains where a command came from: which executable your shell finds, what it resolves to, and which package manager most likely installed it.

Path conventions are the primary signal. Windows app execution aliases are recognized too. For executables in system directories, `how` also asks an available package database (`dpkg`, RPM, pacman, apk, or FreeBSD pkg) which package owns the file.

Configured installation roots are considered as well. Depending on the provider, `how` reads documented environment variables and configuration files or caches a read-only query such as `rustup which`, `pnpm bin --global`, `go env`, `uv tool dir`, or `pipx environment`.

Configured aliases from bash, zsh, fish, Nushell, PowerShell, and tcsh/csh are expanded before `PATH` is searched, including aliases that add arguments or point to another alias.

## Install

The recommended option is [`cargo-binstall`](https://github.com/cargo-bins/cargo-binstall),
which uses a prebuilt binary when one is available:

```console
cargo binstall how
```

Alternatively, build and install the crate from crates.io with Cargo:

```console
cargo install how --locked
```

Or install the default package from this repository with Nix:

```console
nix profile install github:George-Miao/how
```

The Nix flake supports `aarch64-darwin`, `aarch64-linux`, and `x86_64-linux`.

Prebuilt release archives, including those used by `cargo-binstall`, are
available for these exact Rust targets:

| Platform | Architecture | Target |
| --- | --- | --- |
| Linux (GNU) | x86-64 | `x86_64-unknown-linux-gnu` |
| Linux (GNU) | ARM64 | `aarch64-unknown-linux-gnu` |
| Linux (musl) | x86-64 | `x86_64-unknown-linux-musl` |
| Linux (musl) | ARM64 | `aarch64-unknown-linux-musl` |
| macOS | x86-64 | `x86_64-apple-darwin` |
| macOS | ARM64 | `aarch64-apple-darwin` |
| Windows (MSVC) | x86-64 | `x86_64-pc-windows-msvc` |

To install manually, download the matching archive from the
[latest GitHub release](https://github.com/George-Miao/how/releases/latest),
verify it against `SHA256SUMS`, extract `how` (`how.exe` on Windows), and move
the binary into a directory on your `PATH`.

## Use

```console
$ how rg
/opt/homebrew/bin/rg
  → /opt/homebrew/Cellar/ripgrep/14.1.1/bin/rg
  manager      Homebrew
  package      ripgrep
  version      14.1.1
  confidence   high
  evidence
    • path convention: target lives in a Homebrew Cellar
    • symlink: target is /opt/homebrew/Cellar/ripgrep/14.1.1/bin/rg
```

Help and human-readable results use color when their output stream supports it.
Redirected output and JSON remain unstyled.

Inspect shadowed commands too:

```console
how --all python
```

Choose a supported shell explicitly when `SHELL` does not identify the shell
whose aliases should be inspected:

```console
how --shell /bin/zsh ll
how --shell pwsh gci
```

Disable alias expansion for a direct `PATH` lookup:

```console
how --no-aliases ll
```

Produce stable, machine-readable output:

```console
how --json rg
```

The JSON document has an explicit version and keeps every provenance field
present, using `null` when that detail is unknown:

```json
{
  "schema_version": 1,
  "installations": [
    {
      "executable": "/opt/homebrew/bin/rg",
      "resolved": "/opt/homebrew/Cellar/ripgrep/14.1.1/bin/rg",
      "manager": "Homebrew",
      "package": "ripgrep",
      "version": "14.1.1",
      "environment": null,
      "toolchain": null,
      "derivation": null,
      "confidence": "high",
      "evidence": [
        {
          "kind": "path convention",
          "detail": "target lives in a Homebrew Cellar"
        }
      ]
    }
  ]
}
```

`package` is reserved for an installed package. `environment` identifies a
Conda-compatible or Pixi environment, `toolchain` identifies a runtime managed
by tools such as mise, asdf, pyenv, or rbenv, `version` records a detected
package or toolchain version, and `derivation` identifies a Nix store
derivation. Consumers should reject unsupported `schema_version` values; the
version changes when the JSON contract changes incompatibly.

Explain how matching candidates were ranked:

```console
how --explain rg
```

The explanation names the selected candidate, every rejected candidate that was
safely considered, and the selection reason. Candidates are ranked by
confidence, explicit provider priority, then deterministic phase and candidate
tie-breaks. Dynamic and package-database probes remain lazy: probing stops once
later candidates cannot outrank the current result. `--explain` and `--json`
currently conflict because the stable JSON schema does not include arbitration
details.

## Supported detection

Every path provider checks both the executable found on `PATH` and its resolved
symlink target. "Default" below means the package manager's conventional
per-user location. Environment variables and read-only command queries are used
only to discover paths; `how` does not change package-manager configuration.

| Provider | Typical systems | How it is detected | Configured path support |
| --- | --- | --- | --- |
| Nix | Linux, macOS | Store path such as `/nix/store/<hash>-<package>/...` | `NIX_STORE_DIR` |
| Homebrew | macOS, Linux | Formula beneath a Homebrew `Cellar` | `HOMEBREW_CELLAR`; otherwise `brew --cellar` |
| Snap | Linux | `/snap/bin` exposure or a mounted `/snap/<package>/...` path | — (path convention) |
| Flatpak | Linux | A `flatpak/exports/bin` path | — (path convention) |
| MacPorts | macOS | `/opt/local/...` prefix | — (fixed prefix) |
| Pixi | Cross-platform | Workspace or global environments, plus executables exposed from Pixi home | `PIXI_HOME` or default |
| Conda-compatible | Cross-platform | A prefix containing `conda-meta` (Conda, Mamba, or Micromamba) | Prefix metadata supports named and custom environments |
| mise | Cross-platform | Tool installs or shims beneath the mise data directory | `MISE_DATA_DIR`, `XDG_DATA_HOME`, or default |
| asdf | Unix-like | Tool installs or shims beneath the asdf data directory | `ASDF_DATA_DIR` or default |
| pyenv | Unix-like | Python versions or shims beneath the pyenv root | `PYENV_ROOT`; otherwise `pyenv root` or default |
| rbenv | Unix-like | Ruby versions or shims beneath the rbenv root | `RBENV_ROOT`; otherwise `rbenv root` or default |
| Volta | Cross-platform | Package images or shims beneath the Volta home | `VOLTA_HOME` or default |
| uv | Cross-platform | Installed tool directory or an explicitly configured tool bin directory | `UV_TOOL_DIR`, `UV_TOOL_BIN_DIR`, `uv tool dir`, `XDG_DATA_HOME`, or default |
| pipx | Cross-platform | Managed virtual environments or explicitly configured bin directories | `PIPX_HOME`, `PIPX_GLOBAL_HOME`, `PIPX_BIN_DIR`, `PIPX_GLOBAL_BIN_DIR`, `pipx environment`, or platform defaults |
| pnpm | Cross-platform | `.pnpm` virtual-store layout or a global bin directory | `PNPM_HOME`; otherwise `pnpm bin --global` or default |
| npm | Cross-platform | `node_modules` layout or the global prefix's `bin` directory | `NPM_CONFIG_PREFIX`; otherwise `npm prefix --global` |
| Yarn | Cross-platform | A path containing a `.yarn` managed directory | — (path convention) |
| Bun | Cross-platform | `bin` beneath the Bun install root | `BUN_INSTALL` or default |
| Deno | Cross-platform | `bin` beneath the Deno install root | `DENO_INSTALL_ROOT` or default |
| Composer | Cross-platform | `vendor/bin` beneath Composer home | `COMPOSER_HOME` or default |
| Rustup | Cross-platform | Direct toolchain `bin` paths or standard proxies verified with `rustup which`; `rustup` itself is verified with `rustup --version` | `RUSTUP_HOME`, `CARGO_HOME`, or defaults |
| Cargo | Cross-platform | `bin` beneath the Cargo install root | `CARGO_INSTALL_ROOT`, `CARGO_HOME`, or `install.root` in Cargo config; otherwise default |
| Go | Cross-platform | A Go install `bin` directory | `GOBIN`, `GOPATH`, `go env GOBIN`, `go env GOPATH`, or default |
| MSYS2 pacman | Windows | MSYS/UCRT/CLANG/MinGW prefix layout; then `pacman -Qqo` when available | Root is inferred from the executable, including non-default drives and directories |
| WinGet | Windows | Portable-package roots or the WinGet links directory | Locations derived from `LOCALAPPDATA` and `PROGRAMFILES` |
| Scoop | Windows | Apps or shims beneath Scoop roots | `SCOOP`, `SCOOP_GLOBAL`, `PROGRAMDATA`, or default |
| Chocolatey | Windows | Packages or shims beneath the Chocolatey root | `ChocolateyInstall`, `PROGRAMDATA`, or default |
| Microsoft Store / App Installer | Windows | App execution alias in `Microsoft/WindowsApps` | Location derived from `LOCALAPPDATA` |
| System fallback | Unix-like | `/usr/bin`, `/usr/sbin`, `/bin`, or `/sbin` | — (fixed prefixes; low-confidence fallback) |

Rustup is checked before Cargo because both use `CARGO_HOME/bin`. Only `rustup`
and the standard Rustup proxy names are queried and must verify successfully;
other binaries in that directory remain Cargo-installed candidates.

When no high-confidence path convention matches, `how` asks each available
native package database about the exact resolved executable:

| Package database | Typical systems | Ownership query |
| --- | --- | --- |
| dpkg | Debian, Ubuntu, and derivatives | `dpkg-query -S <path>` |
| RPM | Fedora, RHEL, openSUSE, and derivatives | `rpm -qf <path>` |
| pacman | Arch Linux and derivatives | `pacman -Qqo <path>` |
| apk | Alpine Linux | `apk info --who-owns <path>` |
| FreeBSD pkg | FreeBSD | `pkg which -q <path>` |

Shell aliases are supported for zsh, bash, fish, Nushell (`nu`), PowerShell
(`pwsh` and Windows PowerShell), and tcsh/csh. By default, `how` uses the shell
named by `SHELL`; alias chains are followed, safe `env` and `command` wrappers
are unwrapped, and only the aliased command target—not its arguments—is
resolved. Use `--shell <program-or-path>` to select a supported shell explicitly
or `--no-aliases` to skip alias queries.

On Windows, `SHELL` is still the only automatic shell signal. When it is absent,
alias resolution is skipped rather than assuming PowerShell, because the
current process may have been launched from Command Prompt, PowerShell, or a
Unix-compatible environment. Pass `--shell pwsh` or `--shell powershell` when
PowerShell alias resolution is desired.
