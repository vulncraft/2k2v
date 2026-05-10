{
  description = "Distributed KV store";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    crane.url = "github:ipetkov/crane";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, crane, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};

        # musl toolchain for a fully static binary
        muslTarget = pkgs.pkgsCross.musl64;
        craneLib = (crane.mkLib muslTarget);

        commonArgs = {
          src = craneLib.cleanCargoSource ./.;
          strictDeps = true;

          CARGO_BUILD_TARGET = "x86_64-unknown-linux-musl";
          CARGO_BUILD_RUSTFLAGS = "-C target-feature=+crt-static";
        };

        my-crate = craneLib.buildPackage (commonArgs // {
          cargoArtifacts = craneLib.buildDepsOnly commonArgs;
        });

        docker-image = pkgs.dockerTools.buildLayeredImage {
          name = "kvnode";
          tag = "latest";

          # fakeNss provides /etc/passwd and /etc/group so getpwuid() doesn't panic
          contents = [ pkgs.dockerTools.fakeNss pkgs.tini ];

          config = {
            Entrypoint = [ "${pkgs.tini}/bin/tini" "--" "${my-crate}/bin/kvnode" ];
            User = "nobody";
          };
        };
      in
      {
        packages.default = my-crate;
        packages.docker = docker-image;

        apps.default = flake-utils.lib.mkApp { drv = my-crate; };

        devShells.default = craneLib.devShell {
          checks = self.checks.${system};
        };
      });
}
