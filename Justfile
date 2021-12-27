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

sqlx-prep:
  cargo sqlx prepare --merged

build:
  cargo build --no-default-features --features=stable-release

build-release: check
  cargo build --release --no-default-features --features=stable-release

build-release-docker: sqlx-prep
  docker run -it --rm -v $PWD:/app rust:1.57-buster /bin/bash /app/docker-build.sh

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

test: check
  cargo test --workspace

fulltest: fullcheck
  cargo hack test {{hack_step_a}}
  cargo hack test {{hack_step_b}}

devdb:
  docker-compose up -d postgres