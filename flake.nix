{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
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
      rustVersion = pkgs.rust-bin.stable.latest.default;
      rustPlatform = pkgs.makeRustPlatform {
        cargo = rustVersion;
        rustc = rustVersion;
      };
      linuxBuildInputs = [
        pkgs.wayland
      ];
      cursedboard = rustPlatform.buildRustPackage {
        pname = "cursedboard";
        version = "0.1.0";
        src = ./.;
        cargoLock.lockFile = ./Cargo.lock;

        nativeBuildInputs = pkgs.lib.optionals pkgs.stdenv.hostPlatform.isLinux [
          pkgs.pkg-config
        ];

        buildInputs =
          pkgs.lib.optionals pkgs.stdenv.hostPlatform.isLinux linuxBuildInputs
          ++ pkgs.lib.optionals pkgs.stdenv.hostPlatform.isDarwin [
            pkgs.apple-sdk_15
          ];
      };
    in {
      packages = {
        default = cursedboard;
        inherit cursedboard;
      };

      devShells.default = pkgs.mkShell {
        packages = [
          rustVersion
        ];

        buildInputs =
          pkgs.lib.optionals pkgs.stdenv.hostPlatform.isLinux linuxBuildInputs
          ++ pkgs.lib.optionals pkgs.stdenv.hostPlatform.isDarwin [
            pkgs.apple-sdk_15
          ];
      };
    }) // {
      nixosModules.default = import ./nix/module.nix;
    };
}
