{
  description = "arma-3-status-bot";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";

    fenix.url = "github:nix-community/fenix";
    fenix.inputs.nixpkgs.follows = "nixpkgs";

    cargo2nix.url = "github:cargo2nix/cargo2nix/release-0.11.0";
    cargo2nix.inputs.flake-utils.follows = "flake-utils";
    cargo2nix.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs =
    { self
    , nixpkgs
    , flake-utils
    , ...
    } @ inputs:
    let
      pkgsFor = system: import nixpkgs {
        inherit system;
        overlays = [
          inputs.cargo2nix.overlays.default
          inputs.fenix.overlays.default

          (final: prev: {
            rust-toolchain =
              let
                inherit (final.lib) fakeSha256;
                inherit (final.lib.strings) fileContents;

                toolchainFor = target: target.fromToolchainFile {
                  file = ./rust-toolchain.toml;
                  # Replace `fakeSha256` with the hash string produced by Nix
                  # when it tries to build this for the first time.
                  sha256 = "sha256-SXRtAuO4IqNOQq+nLbrsDFbVk+3aVA8NNpSZsKlVH/8=";
                };

                rustfmt = final.fenix.latest.rustfmt;
              in
              final.fenix.combine [
                rustfmt
                (toolchainFor final.fenix)
              ];
          })

          (final: prev: {
            cargo2nix = inputs.cargo2nix.packages.${system}.default;
          })
        ];
      };

      supportedSystems = with flake-utils.lib.system; [
        aarch64-darwin
        x86_64-darwin
        x86_64-linux
      ];

      inherit (flake-utils.lib) eachSystem;
    in
    eachSystem supportedSystems (system:
    let
      pkgs = pkgsFor system;

      rustPkgs = pkgs.rustBuilder.makePackageSet {
        packageFun = import ./Cargo.nix;
        rustToolchain = pkgs.rust-toolchain;
      };
    in
    rec
    {
      packages = rec {
        default = arma-3-status-bot;
        arma-3-status-bot = (rustPkgs.workspace.arma-3-status-bot { }).bin;
      };

      apps = rec {
        default = arma-3-status-bot;
        arma-3-status-bot = flake-utils.lib.mkApp {
          drv = packages.arma-3-status-bot;
        };
      };

      devShells.default = pkgs.mkShell {
        packages = with pkgs; [
          cargo2nix
          rust-toolchain

          libiconv
        ];
      };

      formatter = pkgs.nixpkgs-fmt;
    });
}
