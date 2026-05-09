{
  description = "Distributed KV store";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";

    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    crane.url = "github:ipetkov/crane";
  };

  outputs = { self, nixpkgs, flake-utils, fenix, crane }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};

        toolchain = fenix.packages.${system}.fromToolchainFile {
          file = ./rust-toolchain.toml;
          sha256 = "sha256-gh/xTkxKHL4eiRXzWv8KP7vfjSk61Iq48x47BEDFgfk=";
        };

        craneLib = (crane.mkLib pkgs).overrideToolchain toolchain;

        myPackage = craneLib.buildPackage {
          src = craneLib.cleanCargoSource ./.;
          strictDeps = true;
        };

        myTests = craneLib.cargoNextest {
          src = craneLib.cleanCargoSource ./.;
          strictDeps = true;
          cargoArtifacts = craneLib.buildDepsOnly {
            src = craneLib.cleanCargoSource ./.;
          };
        };
      in {
        packages.default = myPackage;

        checks = {
          inherit myPackage;
          tests = myTests;
        };

        devShells.default = craneLib.devShell {
          packages = with pkgs; [
            just
            rust-analyzer
            cargo-watch
            cargo-nextest
          ];
        };
      });
}
