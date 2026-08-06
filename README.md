# how

`how` explains where a command came from: which executable your shell finds, what it resolves to, and which package manager most likely installed it.

Path conventions are the primary signal. Windows app execution aliases are recognized too. For executables in system directories, `how` also asks an available package database (`dpkg`, RPM, pacman, apk, or FreeBSD pkg) which package owns the file.

Configured installation roots are considered as well. Depending on the provider, `how` reads documented environment variables and configuration files or caches a read-only query such as `pnpm bin --global`, `go env`, `uv tool dir`, or `pipx environment`.

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
  confidence   high confidence
  evidence
    • path convention — target lives in a Homebrew Cellar
    • symlink — target is /opt/homebrew/Cellar/ripgrep/14.1.1/bin/rg
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
