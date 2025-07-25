{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable"; # We want to use packages from the binary cache
    flake-utils.url = "github:numtide/flake-utils";
    # Rust overlay
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = inputs @ {
    self,
    nixpkgs,
    flake-utils,
    rust-overlay,
    ...
  }:
    flake-utils.lib.eachSystem [
      "x86_64-linux"
      "aarch64-linux"
      "aarch64-darwin"
    ] (system: let
      overlays = [(import rust-overlay)];
      pkgs = import nixpkgs {
        inherit system overlays;
      };
      rustVersion = pkgs.rust-bin.fromRustupToolchainFile ./macos/rust-toolchain.toml;
      rustPlatform = pkgs.makeRustPlatform {
        cargo = rustVersion;
        rustc = rustVersion;
      };
      appRustBuildMacOS = rustPlatform.buildRustPackage {
        pname = "macos";
        version = "0.1";
        src = ./.;
        cargoLock.lockFile = ./Cargo.lock;
        buildAndTestSubdir = "macos";
      };
      appRustBuildLinux = rustPlatform.buildRustPackage {
        pname = "linux";
        version = "0.1";
        src = ./.;
        cargoLock.lockFile = ./Cargo.lock;
        buildAndTestSubdir = "linux";
      };
    in {
      # For `nix build` & `nix run`:
      packages = {
        macos = appRustBuildMacOS;
        linux = appRustBuildLinux;

        # default = appRustBuild;

        inherit (pkgs) rust-toolchain;
      };

      devShell = pkgs.mkShell {
        packages = [
          pkgs.rust-bin.stable.latest.default
          pkgs.zls
        ];

        buildInputs = with pkgs; [
          rust-bin.stable.latest.default
        ];
      };
    });
}
