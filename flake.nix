{
  description = "Harmonia — Rust media management system";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    crane.url = "github:ipetkov/crane";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, crane, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };

        lib = pkgs.lib;

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          targets = [ "aarch64-unknown-linux-gnu" ];
        };

        craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;

        # Restrict source to Rust workspace files only. The monorepo also
        # contains mouseion/ (C#), akroasis/ (Kotlin/TS), docs/, and legacy/ —
        # none of which are part of this Cargo workspace and must not invalidate
        # the Nix build cache on every documentation change.
        src = lib.fileset.toSource {
          root = ./.;
          fileset = lib.fileset.unions [
            ./Cargo.toml
            ./Cargo.lock
            ./crates
          ];
        };

        nativeBuildInputs = with pkgs; [
          pkg-config
          cmake
        ];

        buildInputs = with pkgs; [
          alsa-lib  # cpal ALSA backend
          openssl   # reqwest TLS — consolidation to rustls deferred to R5 audit
          sqlite    # sqlx
          libopus   # opus crate FFI
        ];

        commonArgs = {
          inherit src nativeBuildInputs buildInputs;
          strictDeps = true;
          # sqlx compile-time query validation requires sqlx-data.json committed
          # at workspace root. Generate with: cargo sqlx prepare --workspace
          SQLX_OFFLINE = "true";
        };

        # Build workspace dependencies in a separate derivation so that
        # downstream packages share the cached output and don't rebuild on
        # source-only changes.
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        # Cross-compile inputs for aarch64. Sourced from pkgsCross so the
        # linked libraries match the target architecture.
        pkgsCross = import nixpkgs {
          localSystem = system;
          crossSystem.config = "aarch64-unknown-linux-gnu";
        };

        crossLinker =
          "${pkgsCross.stdenv.cc}/bin/aarch64-unknown-linux-gnu-gcc";

        crossBuildInputs = with pkgsCross; [
          alsa-lib
          openssl
          sqlite
          libopus
        ];

        crossArtifacts = craneLib.buildDepsOnly (commonArgs // {
          CARGO_BUILD_TARGET = "aarch64-unknown-linux-gnu";
          CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER = crossLinker;
          HOST_CC = "${pkgs.stdenv.cc}/bin/cc";
          buildInputs = crossBuildInputs;
          nativeBuildInputs = nativeBuildInputs ++ [ pkgsCross.stdenv.cc ];
          PKG_CONFIG_ALLOW_CROSS = "1";
        });

      in {
        packages = {
          default = craneLib.buildPackage (commonArgs // {
            inherit cargoArtifacts;
          });

          # Cross-compiled binary for Raspberry Pi renderer nodes.
          # On a non-arm host, requires binfmt QEMU support to run the output.
          # Build with: nix build .#harmonia-aarch64
          harmonia-aarch64 = craneLib.buildPackage (commonArgs // {
            cargoArtifacts = crossArtifacts;
            CARGO_BUILD_TARGET = "aarch64-unknown-linux-gnu";
            CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER = crossLinker;
            HOST_CC = "${pkgs.stdenv.cc}/bin/cc";
            buildInputs = crossBuildInputs;
            nativeBuildInputs = nativeBuildInputs ++ [ pkgsCross.stdenv.cc ];
            PKG_CONFIG_ALLOW_CROSS = "1";
          });
        };

        devShells.default = craneLib.devShell {
          inputsFrom = [ self.packages.${system}.default ];
          packages = with pkgs; [
            rust-analyzer
            cargo-watch
            cargo-deny
            cargo-nextest
            sqlx-cli
          ];
        };

        checks = {
          clippy = craneLib.cargoClippy (commonArgs // {
            inherit cargoArtifacts;
            cargoClippyExtraArgs = "-- -D warnings";
          });

          tests = craneLib.cargoNextest (commonArgs // {
            inherit cargoArtifacts;
          });

          fmt = craneLib.cargoFmt commonArgs;

          deny = craneLib.cargoDeny commonArgs;
        };
      }
    );
}
