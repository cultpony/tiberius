set dotenv-load := true
msrv := "1.56"
maxrv := ""
excluded_features_all := ",default,dev-release"
excluded_features_step_a := "full-release" + excluded_features_all
excluded_features_step_b := "stable-release" + excluded_features_all

hack_step := "--all --feature-powerset --version-range " + msrv + ".." + maxrv + " --exclude-features "
hack_step_a := hack_step + excluded_features_step_a
hack_step_b := hack_step + excluded_features_step_b

run: build
  cargo run --no-default-features --features=stable-release -- server -z

run-release: build
  cargo run --release --no-default-features --features=stable-release -- server -z

sqlx-prep:
  cargo sqlx prepare --merged

build:
  cargo build --no-default-features --features=stable-release

build-release:
  cargo build --release --no-default-features --features=stable-release

build-release-ubuntu: sqlx-prepare
  cross build --release

sqlx-prepare:
  cargo sqlx prepare --merged

fullbuild: fullcheck
  cargo hack build {{hack_step_a}}
  cargo hack build {{hack_step_b}}

run-full:
  cargo run -- server -z

check:
  cargo check --workspace --no-default-features --features=stable-release

fullcheck:
  cargo hack check {{hack_step_a}}
  cargo hack check {{hack_step_b}}

fmt:
  cargo fmt

clippy:
  cargo clippy --workspace

test:
  cargo test --workspace

test-release:
  cargo test --workspace --release

fulltest: fullcheck
  cargo hack test {{hack_step_a}}
  cargo hack test {{hack_step_b}}

devdb:
  docker-compose up -d postgres

audit:
  cargo audit --ignore RUSTSEC-2020-0071 --ignore RUSTSEC-2022-0006 --ignore RUSTSEC-2022-0013 --ignore RUSTSEC-2021-0127
