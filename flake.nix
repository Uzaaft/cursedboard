{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable"; # We want to use packages from the binary cache
    flake-utils.url = "github:numtide/flake-utils";
    # Rust overlay
    rust-overlay.url = "github:oxalica/rust-overlay";
    # Zig overlay
    zig.url = "github:mitchellh/zig-overlay";
  };

  outputs = inputs @ {
    self,
    nixpkgs,
    flake-utils,
    rust-overlay,
    zig,
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
      appRustBuild = rustPlatform.buildRustPackage {
        pname = "macos";
        version = "0.1.0";
        src = ./macos;
        cargoLock.lockFile = ./macos/Cargo.lock;
      };
    in {
      # For `nix build` & `nix run`:
      packages = {
        macos = appRustBuild;

        # default = appRustBuild;

        inherit (pkgs) rust-toolchain;
      };

      devShell = pkgs.mkShell {
        packages = [
          zig.packages.${system}."0.14.0"
          pkgs.rust-bin.stable.latest.default
          pkgs.zls
        ];

        buildInputs = with pkgs; [
          rust-bin.stable.latest.default
          zig.packages.${system}."0.14.0"
        ];
      };
    });
}
