lint:
  cargo clippy -- -Dwarnings

build-image:
  nix build .#docker
  docker load < result

[working-directory: 'system_test']
behave:
  TEST_BINARY=../target/debug/kvnode uv run behave features/ --junit
