# how

`how` explains where a command came from: which executable your shell finds, what
it resolves to, and which package manager most likely installed it.

Path conventions are the primary signal. This makes detection quick and useful
for tools installed by Homebrew, Nix, Cargo, Go, npm/pnpm/Yarn/Bun, pipx, uv,
mise, asdf, pyenv, rbenv, Volta, Snap, Flatpak, MacPorts, WinGet, Scoop, and
Chocolatey. Windows app execution aliases are recognized too. For executables
in system directories, `how` also asks an available package database (`dpkg`,
RPM, pacman, apk, or FreeBSD pkg) which package owns the file.

Configured installation roots are considered as well as conventional paths.
Depending on the provider, `how` reads documented environment variables and
configuration files or caches a read-only query such as `pnpm bin --global`,
`go env`, `uv tool dir`, or `pipx environment`.

## Install

```console
cargo install --path .
```

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

`how` inspects executables, not shell state, so aliases, functions, builtins, and
hashed shell command locations are outside its scope. A medium or low confidence
result is an inference rather than proof; the evidence lines show what it used.

## Add a provider

Package-manager logic lives in `src/provider/<name>.rs`. Each module defines its
own concrete type, a static instance, and an implementation of the shared
`Provider` trait. The registries in `src/provider/mod.rs` contain
`&'static dyn Provider` references, so detection performs no provider allocation.

Shared path, configuration, and ownership-query helpers live in `src/util.rs`.
Register a new static provider in `src/provider/mod.rs`; detection and output
code do not need package-manager-specific branches.
