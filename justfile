lint:
  cargo clippy -- -Dwarnings

build-image:
  nix build .#docker
  docker load < result
