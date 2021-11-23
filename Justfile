set dotenv-load := true


run: build
  cargo run --no-default-features --features=stable-release -- server -z

build: check
  cargo build --no-default-features --features=stable-release

run-full:
  cargo run -- server -z

check:
  cargo check --workspace

fmt:
  cargo fmt

clippy:
  cargo clippy --workspace

test: build
  cargo test --workspace

devdb:
  docker-compose up -d postgres