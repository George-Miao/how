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

Configured installation roots are considered as well. Depending on the provider, `how` reads documented environment variables and configuration files or caches a read-only query such as `pnpm bin --global`, `go env`, `uv tool dir`, or `pipx environment`.

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

The flake supports `aarch64-darwin`, `aarch64-linux`, and `x86_64-linux`.

To install manually, download the archive for your platform from the
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

Produce stable, machine-readable output:

```console
how --json rg
```
