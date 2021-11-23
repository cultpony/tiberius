set dotenv-load := true
msrv := "1.56"
excluded_features_step_a := "full-release,default"
excluded_features_step_b := "stable-release,default"

hack_step := "--all --feature-powerset --version-range " + msrv + " --exclude-features "
hack_step_a := hack_step + excluded_features_step_a
hack_step_b := hack_step + excluded_features_step_b

run: build
  cargo run --no-default-features --features=stable-release -- server -z

build: check
  cargo build --no-default-features --features=stable-release

fullbuild: fullcheck
  cargo hack build {{hack_step_a}}
  cargo hack build {{hack_step_b}}

run-full:
  cargo run -- server -z

check:
  cargo check --workspace

fullcheck:
  cargo hack check {{hack_step_a}}
  cargo hack check {{hack_step_b}}

fmt:
  cargo fmt

clippy:
  cargo clippy --workspace

test: build
  cargo test --workspace

fulltest: fullbuild
  cargo hack test {{hack_step_a}}
  cargo hack test {{hack_step_b}}

devdb:
  docker-compose up -d postgres