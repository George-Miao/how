{
  description = "Detect how a command was installed";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
    crane.url = "github:ipetkov/crane";
    rust-overlay.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs =
    {
      nixpkgs,
      rust-overlay,
      flake-utils,
      crane,
      ...
    }:
    flake-utils.lib.eachSystem
      [
        "aarch64-darwin"
        "aarch64-linux"
        "x86_64-linux"
      ]
      (
        system:
        let
          overlays = [ (import rust-overlay) ];
          pkgs = import nixpkgs {
            inherit system overlays;
          };
          manifest = builtins.fromTOML (builtins.readFile ./Cargo.toml);
          rustVersion =
            if builtins.match "[0-9]+\\.[0-9]+" manifest.package.rust-version != null then
              "${manifest.package.rust-version}.0"
            else
              manifest.package.rust-version;
          craneLib = (crane.mkLib pkgs).overrideToolchain (p: p.rust-bin.stable.${rustVersion}.default);
          src = craneLib.cleanCargoSource ./.;
          commonArgs = {
            inherit src;
            pname = "how";
            version = manifest.package.version;
            strictDeps = true;

            meta = {
              description = "How was a command installed?";
              homepage = "https://github.com/George-Miao/how";
              license = with pkgs.lib.licenses; [
                asl20
                mit
              ];
              mainProgram = "how";
            };
          };
          cargoArtifacts = craneLib.buildDepsOnly commonArgs;
          how = craneLib.buildPackage (commonArgs // { inherit cargoArtifacts; });
        in
        with pkgs;
        {
          packages = {
            inherit how;
            default = how;
          };

          devShells.default = mkShell {
            buildInputs = [
              (rust-bin.selectLatestNightlyWith (
                toolchain:
                toolchain.default.override {
                  extensions = [
                    "rust-src"
                  ];
                  targets = [
                    "x86_64-unknown-linux-gnu"
                    "x86_64-unknown-freebsd"
                    "x86_64-pc-windows-gnu"
                    "aarch64-apple-darwin"
                  ];
                }
              ))
            ];
          };
        }
      );
}
